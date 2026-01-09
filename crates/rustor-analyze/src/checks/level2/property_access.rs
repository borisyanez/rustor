//! Check for access to undefined properties on known types (Level 2)

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::{HashMap, HashSet};

/// Checks for property access on objects where we know the type
pub struct PropertyAccessCheck;

impl Check for PropertyAccessCheck {
    fn id(&self) -> &'static str {
        "property.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects property access on known types where property doesn't exist"
    }

    fn level(&self) -> u8 {
        2
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = PropertyAccessVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_properties: HashMap::new(),
            class_names: HashMap::new(),
            variable_types: HashMap::new(),
            param_type_stack: Vec::new(),
            builtin_classes: ctx.builtin_classes,
            issues: Vec::new(),
        };

        // First pass: collect class properties
        visitor.collect_definitions(program);

        // Second pass: check property access (with function scope tracking)
        visitor.analyze_program(program);

        visitor.issues
    }
}

struct PropertyAccessVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    class_properties: HashMap<String, HashSet<String>>, // class name (lowercase) -> property names
    class_names: HashMap<String, String>,               // class name (lowercase) -> original name
    variable_types: HashMap<String, String>,             // variable name -> class name (original)
    /// Stack of parameter types for nested function scopes
    param_type_stack: Vec<HashMap<String, String>>,
    builtin_classes: &'s [&'static str],
    issues: Vec<Issue>,
}

impl<'s> PropertyAccessVisitor<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_from_stmt(stmt);
        }
    }

    fn collect_from_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                let original_name = self.get_span_text(&class.name.span).to_string();
                let class_lower = original_name.to_lowercase();
                let mut properties = HashSet::new();

                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Property(prop) => {
                            // Use the variables() method to get all property variables
                            for var in prop.variables() {
                                let prop_name = self.get_span_text(&var.span);
                                // Remove $ prefix if present
                                let name = prop_name.trim_start_matches('$').to_lowercase();
                                properties.insert(name);
                            }
                        }
                        // Also collect promoted properties from constructor
                        ClassLikeMember::Method(method) => {
                            let method_name = self.get_span_text(&method.name.span).to_lowercase();
                            if method_name == "__construct" {
                                for param in method.parameter_list.parameters.iter() {
                                    if param.is_promoted_property() {
                                        let prop_name = self.get_span_text(&param.variable.span);
                                        let name = prop_name.trim_start_matches('$').to_lowercase();
                                        properties.insert(name);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                self.class_names.insert(class_lower.clone(), original_name);
                self.class_properties.insert(class_lower, properties);
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_stmt(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_stmt(inner);
                    }
                }
            },
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_from_stmt(inner);
                }
            }
            _ => {}
        }
    }

    fn get_line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// Extract class name from an instantiation expression
    fn get_instantiation_class<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Instantiation(inst) => match &*inst.class {
                Expression::Identifier(ident) => {
                    Some(self.get_span_text(&ident.span()).to_string())
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Get variable type from parameter stack or variable_types map
    fn get_variable_type(&self, var_name: &str) -> Option<String> {
        // First check parameter stack (innermost scope first)
        for scope in self.param_type_stack.iter().rev() {
            if let Some(type_name) = scope.get(var_name) {
                return Some(type_name.clone());
            }
        }
        // Then check variable assignments
        self.variable_types.get(var_name).cloned()
    }

    /// Extract typed parameters from a parameter list
    fn extract_typed_params(&self, params: &FunctionLikeParameterList<'_>) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for param in params.parameters.iter() {
            if let Some(hint) = &param.hint {
                // Get the type name from the hint
                if let Some(type_name) = self.extract_type_name(hint) {
                    let var_name = self.get_span_text(&param.variable.span).to_string();
                    result.insert(var_name, type_name);
                }
            }
        }
        result
    }

    /// Extract type name from a Hint
    fn extract_type_name(&self, hint: &Hint<'_>) -> Option<String> {
        match hint {
            Hint::Identifier(ident) => Some(self.get_span_text(&ident.span()).to_string()),
            Hint::Nullable(nullable) => self.extract_type_name(&nullable.hint),
            Hint::Parenthesized(p) => self.extract_type_name(&p.hint),
            _ => None, // Skip union types, intersection types, etc. for now
        }
    }

    /// Analyze program with scope tracking for typed parameters
    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                // Push parameter types onto stack
                let params = self.extract_typed_params(&func.parameter_list);
                self.param_type_stack.push(params);

                // Analyze function body
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
                }

                // Pop parameter types
                self.param_type_stack.pop();
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            // Push parameter types onto stack
                            let params = self.extract_typed_params(&method.parameter_list);
                            self.param_type_stack.push(params);

                            // Analyze method body
                            for inner in body.statements.iter() {
                                self.analyze_statement(inner);
                            }

                            // Pop parameter types
                            self.param_type_stack.pop();
                        }
                    }
                }
            }
            Statement::Expression(expr_stmt) => {
                // Track variable assignments
                if let Expression::Assignment(assign) = &expr_stmt.expression {
                    if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                        let var_name = self.get_span_text(&var.span).to_string();
                        if let Some(class_name) = self.get_instantiation_class(assign.rhs) {
                            self.variable_types.insert(var_name, class_name);
                        }
                    }
                }
                self.analyze_expression(&expr_stmt.expression);
            }
            Statement::If(if_stmt) => {
                self.analyze_expression(&if_stmt.condition);
                self.analyze_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.analyze_expression(&while_stmt.condition);
                self.analyze_while_body(&while_stmt.body);
            }
            Statement::For(for_stmt) => {
                self.analyze_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.analyze_expression(&foreach.expression);
                self.analyze_foreach_body(&foreach.body);
            }
            Statement::Try(try_stmt) => {
                for inner in try_stmt.block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for catch in try_stmt.catch_clauses.iter() {
                    for inner in catch.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    for inner in finally.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.analyze_expression(value);
                }
            }
            Statement::Echo(echo) => {
                for value in echo.values.iter() {
                    self.analyze_expression(value);
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            },
            _ => {}
        }
    }

    fn analyze_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.analyze_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition);
                    self.analyze_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.analyze_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition);
                    for inner in else_if.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            }
        }
    }

    fn analyze_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => self.analyze_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => self.analyze_statement(stmt),
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => self.analyze_statement(stmt),
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_expression<'a>(&mut self, expr: &Expression<'a>) {
        // Check for $obj->property access
        if let Expression::Access(Access::Property(prop_access)) = expr {
            self.check_property_access(prop_access);
        }

        // Recurse into nested expressions
        match expr {
            Expression::Call(call) => match call {
                Call::Method(m) => {
                    self.analyze_expression(&m.object);
                }
                Call::NullSafeMethod(n) => {
                    self.analyze_expression(&n.object);
                }
                _ => {}
            },
            Expression::Access(access) => match access {
                Access::Property(p) => self.analyze_expression(&p.object),
                Access::NullSafeProperty(p) => self.analyze_expression(&p.object),
                _ => {}
            },
            Expression::Binary(binary) => {
                self.analyze_expression(&binary.lhs);
                self.analyze_expression(&binary.rhs);
            }
            Expression::UnaryPrefix(p) => self.analyze_expression(&p.operand),
            Expression::UnaryPostfix(p) => self.analyze_expression(&p.operand),
            Expression::Parenthesized(p) => self.analyze_expression(&p.expression),
            Expression::Conditional(t) => {
                self.analyze_expression(&t.condition);
                if let Some(then_expr) = &t.then {
                    self.analyze_expression(then_expr);
                }
                self.analyze_expression(&t.r#else);
            }
            Expression::Assignment(a) => {
                self.analyze_expression(&a.rhs);
            }
            _ => {}
        }
    }

    fn check_property_access<'a>(&mut self, prop_access: &PropertyAccess<'a>) {
        // Get property name
        let prop_info = match &prop_access.property {
            ClassLikeMemberSelector::Identifier(ident) => {
                Some((self.get_span_text(&ident.span).to_string(), ident.span))
            }
            _ => None,
        };

        if let Some((property, prop_span)) = prop_info {
            let prop_lower = property.to_lowercase();

            // Case 1: (new ClassName())->property
            if let Some(class_name) = self.get_instantiation_class(&prop_access.object) {
                self.check_property_on_class(&class_name, &property, &prop_lower, prop_span);
            }
            // Case 2: $obj->property where $obj has a known type
            else if let Expression::Variable(Variable::Direct(var)) = &*prop_access.object {
                let var_name = self.get_span_text(&var.span).to_string();
                if let Some(class_name) = self.get_variable_type(&var_name) {
                    self.check_property_on_class(&class_name, &property, &prop_lower, prop_span);
                }
            }
        }
    }

    fn check_property_on_class(
        &mut self,
        class_name: &str,
        property: &str,
        prop_lower: &str,
        prop_span: mago_span::Span,
    ) {
        // Skip built-in classes
        if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(class_name)) {
            return;
        }

        let class_lower = class_name.to_lowercase();
        if let Some(properties) = self.class_properties.get(&class_lower) {
            if !properties.contains(prop_lower) {
                let (line, col) = self.get_line_col(prop_span.start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "property.notFound",
                        format!(
                            "Access to an undefined property {}::${}.",
                            class_name, property
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("property.notFound"),
                );
            }
        }
    }
}

// Keep the old Visitor impl for backwards compatibility but it's not used anymore
impl<'a, 's> Visitor<'a> for PropertyAccessVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        // Track variable assignments: $obj = new ClassName()
        if let Statement::Expression(expr_stmt) = stmt {
            if let Expression::Assignment(assign) = &expr_stmt.expression {
                // Check if left is a variable and right is an instantiation
                if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                    let var_name = self.get_span_text(&var.span).to_string();
                    if let Some(class_name) = self.get_instantiation_class(assign.rhs) {
                        // Store original class name (type tracking preserves case)
                        self.variable_types.insert(var_name, class_name);
                    }
                }
            }
        }
        true
    }

    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Check for $obj->property access
        if let Expression::Access(Access::Property(prop_access)) = expr {
            // Get property name
            let prop_info = match &prop_access.property {
                ClassLikeMemberSelector::Identifier(ident) => {
                    Some((self.get_span_text(&ident.span).to_string(), ident.span))
                }
                _ => None,
            };

            if let Some((property, prop_span)) = prop_info {
                let prop_lower = property.to_lowercase();

                // Case 1: (new ClassName())->property
                if let Some(class_name) = self.get_instantiation_class(&prop_access.object) {
                    // Skip built-in classes
                    if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&class_name)) {
                        return true;
                    }

                    let class_lower = class_name.to_lowercase();
                    if let Some(properties) = self.class_properties.get(&class_lower) {
                        if !properties.contains(&prop_lower) {
                            let (line, col) = self.get_line_col(prop_span.start.offset as usize);
                            self.issues.push(
                                Issue::error(
                                    "property.notFound",
                                    format!(
                                        "Access to an undefined property {}::${}.",
                                        class_name, property
                                    ),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("property.notFound"),
                            );
                        }
                    }
                }
                // Case 2: $obj->property where $obj was assigned from new ClassName()
                else if let Expression::Variable(Variable::Direct(var)) = &*prop_access.object {
                    let var_name = self.get_span_text(&var.span).to_string();
                    if let Some(class_name) = self.variable_types.get(&var_name) {
                        let class_lower = class_name.to_lowercase();
                        // Skip built-in classes
                        if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&class_name)) {
                            return true;
                        }

                        if let Some(properties) = self.class_properties.get(&class_lower) {
                            if !properties.contains(&prop_lower) {
                                let (line, col) = self.get_line_col(prop_span.start.offset as usize);
                                self.issues.push(
                                    Issue::error(
                                        "property.notFound",
                                        format!(
                                            "Access to an undefined property {}::${}.",
                                            class_name, property
                                        ),
                                        self.file_path.clone(),
                                        line,
                                        col,
                                    )
                                    .with_identifier("property.notFound"),
                                );
                            }
                        }
                    }
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_check_level() {
        let check = PropertyAccessCheck;
        assert_eq!(check.level(), 2);
    }
}
