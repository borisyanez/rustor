//! Check for calls to potentially undefined methods/properties on classes with magic methods (Level 1)
//!
//! At level 1, PHPStan warns about calling undefined methods on classes with `__call`
//! and accessing undefined properties on classes with `__get`.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::{HashMap, HashSet};

/// Information about a class's magic methods and members
#[derive(Debug, Clone, Default)]
struct ClassInfo {
    /// Original class name
    name: String,
    /// Set of defined method names (lowercase)
    methods: HashSet<String>,
    /// Set of defined property names (lowercase)
    properties: HashSet<String>,
    /// Whether the class has __call
    has_call: bool,
    /// Whether the class has __get
    has_get: bool,
}

/// Checks for calls to potentially undefined methods/properties on classes with magic methods
pub struct MagicMethodsCheck;

impl Check for MagicMethodsCheck {
    fn id(&self) -> &'static str {
        "magic.undefined"
    }

    fn description(&self) -> &'static str {
        "Warns about potential undefined methods/properties accessed via magic methods"
    }

    fn level(&self) -> u8 {
        1
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = MagicMethodsVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            classes: HashMap::new(),
            variable_types: HashMap::new(),
            current_class: None,
            builtin_classes: ctx.builtin_classes,
            phpstan_compat: ctx.config.phpstan_compat,
            issues: Vec::new(),
        };

        // First pass: collect class definitions
        visitor.collect_definitions(program);

        // Second pass: check method/property access with class context tracking
        visitor.analyze_program(program);

        visitor.issues
    }
}

struct MagicMethodsVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    classes: HashMap<String, ClassInfo>, // class name (lowercase) -> info
    variable_types: HashMap<String, String>, // variable name -> class name (original)
    current_class: Option<String>,            // current class context (original name)
    builtin_classes: &'s [&'static str],
    phpstan_compat: bool,
    issues: Vec<Issue>,
}

impl<'s> MagicMethodsVisitor<'s> {
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
                let mut info = ClassInfo {
                    name: original_name,
                    ..Default::default()
                };

                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Method(method) => {
                            let method_name = self.get_span_text(&method.name.span).to_string();
                            let method_lower = method_name.to_lowercase();

                            // Check for magic methods
                            if method_lower == "__call" {
                                info.has_call = true;
                            }
                            if method_lower == "__get" {
                                info.has_get = true;
                            }

                            info.methods.insert(method_lower);
                        }
                        ClassLikeMember::Property(prop) => {
                            for var in prop.variables() {
                                let prop_name = self.get_span_text(&var.span);
                                let name = prop_name.trim_start_matches('$').to_lowercase();
                                info.properties.insert(name);
                            }
                        }
                        _ => {}
                    }
                }

                self.classes.insert(class_lower, info);
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

    /// Analyze program with class context tracking
    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();
                self.current_class = Some(class_name);

                // Analyze methods within class
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            for inner in body.statements.iter() {
                                self.analyze_statement(inner);
                            }
                        }
                    }
                }

                self.current_class = None;
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
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
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
            Statement::Function(func) => {
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
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
        match expr {
            // Check for $obj->method() calls
            Expression::Call(Call::Method(call)) => {
                self.check_method_call(call);
                self.analyze_expression(&call.object);
            }
            // Check for $obj->property access
            Expression::Access(Access::Property(prop_access)) => {
                self.check_property_access(prop_access);
                self.analyze_expression(&prop_access.object);
            }
            // Recurse into other expressions
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

    fn check_method_call<'a>(&mut self, call: &MethodCall<'a>) {
        let method_info = match &call.method {
            ClassLikeMemberSelector::Identifier(ident) => {
                Some((self.get_span_text(&ident.span).to_string(), ident.span))
            }
            _ => None,
        };

        if let Some((method, method_span)) = method_info {
            let method_lower = method.to_lowercase();

            // Get class name from: $this, instantiation, or variable
            let class_name = self.get_class_from_object(&call.object);

            if let Some(name) = class_name {
                // Skip built-in classes
                if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&name)) {
                    return;
                }

                let class_lower = name.to_lowercase();
                if let Some(info) = self.classes.get(&class_lower) {
                    // If class has __call and method is not defined, warn
                    if info.has_call && !info.methods.contains(&method_lower) {
                        let (line, col) = self.get_line_col(method_span.start.offset as usize);
                        self.issues.push(
                            Issue::warning(
                                "magic.undefined",
                                format!(
                                    "Call to method {}::{}() which is handled by __call magic method.",
                                    info.name, method
                                ),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("magic.methodCall"),
                        );
                    }
                }
            }
        }
    }

    fn check_property_access<'a>(&mut self, prop_access: &PropertyAccess<'a>) {
        let prop_info = match &prop_access.property {
            ClassLikeMemberSelector::Identifier(ident) => {
                Some((self.get_span_text(&ident.span).to_string(), ident.span))
            }
            _ => None,
        };

        if let Some((property, prop_span)) = prop_info {
            let prop_lower = property.to_lowercase();

            // Get class name from: $this, instantiation, or variable
            let class_name = self.get_class_from_object(&prop_access.object);

            if let Some(name) = class_name {
                // Skip built-in classes
                if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&name)) {
                    return;
                }

                let class_lower = name.to_lowercase();
                if let Some(info) = self.classes.get(&class_lower) {
                    // If class has __get and property is not defined
                    if info.has_get && !info.properties.contains(&prop_lower) {
                        let (line, col) = self.get_line_col(prop_span.start.offset as usize);
                        if self.phpstan_compat {
                            // In phpstan-compat mode, emit error like PHPStan does
                            self.issues.push(
                                Issue::error(
                                    "property.notFound",
                                    format!(
                                        "Access to an undefined property {}::${}.",
                                        info.name, property
                                    ),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("property.notFound"),
                            );
                        } else {
                            // Default: warn about magic method usage
                            self.issues.push(
                                Issue::warning(
                                    "magic.undefined",
                                    format!(
                                        "Access to property {}::${} which is handled by __get magic method.",
                                        info.name, property
                                    ),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("magic.propertyAccess"),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Get class name from an expression that's the object of a method call or property access
    fn get_class_from_object<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        // Check if it's $this
        if let Expression::Variable(Variable::Direct(var)) = expr {
            let var_name = self.get_span_text(&var.span);
            if var_name == "$this" {
                return self.current_class.clone();
            }
            // Check variable types
            return self.variable_types.get(var_name).cloned();
        }

        // Check if it's new ClassName()
        self.get_instantiation_class(expr)
    }
}

impl<'a, 's> Visitor<'a> for MagicMethodsVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        // Track variable assignments: $obj = new ClassName()
        if let Statement::Expression(expr_stmt) = stmt {
            if let Expression::Assignment(assign) = &expr_stmt.expression {
                if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                    let var_name = self.get_span_text(&var.span).to_string();
                    if let Some(class_name) = self.get_instantiation_class(assign.rhs) {
                        self.variable_types.insert(var_name, class_name);
                    }
                }
            }
        }
        true
    }

    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        match expr {
            // Check for $obj->method() calls
            Expression::Call(Call::Method(call)) => {
                let method_info = match &call.method {
                    ClassLikeMemberSelector::Identifier(ident) => {
                        Some((self.get_span_text(&ident.span).to_string(), ident.span))
                    }
                    _ => None,
                };

                if let Some((method, method_span)) = method_info {
                    let method_lower = method.to_lowercase();

                    // Get class name from instantiation or variable
                    let class_name = self.get_instantiation_class(&call.object).or_else(|| {
                        if let Expression::Variable(Variable::Direct(var)) = &*call.object {
                            let var_name = self.get_span_text(&var.span).to_string();
                            self.variable_types.get(&var_name).cloned()
                        } else {
                            None
                        }
                    });

                    if let Some(name) = class_name {
                        // Skip built-in classes
                        if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&name)) {
                            return true;
                        }

                        let class_lower = name.to_lowercase();
                        if let Some(info) = self.classes.get(&class_lower) {
                            // If class has __call and method is not defined, warn
                            if info.has_call && !info.methods.contains(&method_lower) {
                                let (line, col) = self.get_line_col(method_span.start.offset as usize);
                                self.issues.push(
                                    Issue::warning(
                                        "magic.undefined",
                                        format!(
                                            "Call to method {}::{}() which is handled by __call magic method.",
                                            info.name, method
                                        ),
                                        self.file_path.clone(),
                                        line,
                                        col,
                                    )
                                    .with_identifier("magic.methodCall"),
                                );
                            }
                        }
                    }
                }
            }
            // Check for $obj->property access
            Expression::Access(Access::Property(prop_access)) => {
                let prop_info = match &prop_access.property {
                    ClassLikeMemberSelector::Identifier(ident) => {
                        Some((self.get_span_text(&ident.span).to_string(), ident.span))
                    }
                    _ => None,
                };

                if let Some((property, prop_span)) = prop_info {
                    let prop_lower = property.to_lowercase();

                    // Get class name from instantiation or variable
                    let class_name = self.get_instantiation_class(&prop_access.object).or_else(|| {
                        if let Expression::Variable(Variable::Direct(var)) = &*prop_access.object {
                            let var_name = self.get_span_text(&var.span).to_string();
                            self.variable_types.get(&var_name).cloned()
                        } else {
                            None
                        }
                    });

                    if let Some(name) = class_name {
                        // Skip built-in classes
                        if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&name)) {
                            return true;
                        }

                        let class_lower = name.to_lowercase();
                        if let Some(info) = self.classes.get(&class_lower) {
                            // If class has __get and property is not defined
                            if info.has_get && !info.properties.contains(&prop_lower) {
                                let (line, col) = self.get_line_col(prop_span.start.offset as usize);
                                if self.phpstan_compat {
                                    // In phpstan-compat mode, emit error like PHPStan does
                                    self.issues.push(
                                        Issue::error(
                                            "property.notFound",
                                            format!(
                                                "Access to an undefined property {}::${}.",
                                                info.name, property
                                            ),
                                            self.file_path.clone(),
                                            line,
                                            col,
                                        )
                                        .with_identifier("property.notFound"),
                                    );
                                } else {
                                    // Default: warn about magic method usage
                                    self.issues.push(
                                        Issue::warning(
                                            "magic.undefined",
                                            format!(
                                                "Access to property {}::${} which is handled by __get magic method.",
                                                info.name, property
                                            ),
                                            self.file_path.clone(),
                                            line,
                                            col,
                                        )
                                        .with_identifier("magic.propertyAccess"),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_methods_check_level() {
        let check = MagicMethodsCheck;
        assert_eq!(check.level(), 1);
    }
}
