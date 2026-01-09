//! Check for property type validation (Level 3)
//!
//! At level 3, PHPStan checks that values assigned to typed properties match their declared types.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Information about a property
#[derive(Debug, Clone)]
struct PropertyInfo {
    /// Property name
    name: String,
    /// Declared type (if any)
    type_hint: Option<String>,
    /// Whether property is nullable
    is_nullable: bool,
}

/// Checks for property type validation
pub struct PropertyTypeCheck;

impl Check for PropertyTypeCheck {
    fn id(&self) -> &'static str {
        "property.type"
    }

    fn description(&self) -> &'static str {
        "Validates that values assigned to typed properties match their types"
    }

    fn level(&self) -> u8 {
        3
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = PropertyTypeAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_properties: HashMap::new(),
            current_class: None,
            issues: Vec::new(),
        };

        // First pass: collect property definitions
        analyzer.collect_properties(program);

        // Second pass: check assignments
        analyzer.analyze_program(program);

        analyzer.issues
    }
}

struct PropertyTypeAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    class_properties: HashMap<String, HashMap<String, PropertyInfo>>, // class -> property -> info
    current_class: Option<String>,
    issues: Vec<Issue>,
}

impl<'s> PropertyTypeAnalyzer<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
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

    fn collect_properties<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_from_stmt(stmt);
        }
    }

    fn collect_from_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();
                let class_lower = class_name.to_lowercase();
                let mut properties = HashMap::new();

                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Property(prop) => {
                            // Get type hint from property
                            let (type_hint, is_nullable) = self.extract_property_type(prop);

                            for var in prop.variables() {
                                let prop_name = self.get_span_text(&var.span);
                                let name = prop_name.trim_start_matches('$').to_string();
                                let name_lower = name.to_lowercase();

                                properties.insert(
                                    name_lower,
                                    PropertyInfo {
                                        name,
                                        type_hint: type_hint.clone(),
                                        is_nullable,
                                    },
                                );
                            }
                        }
                        // Also handle promoted properties in constructor
                        ClassLikeMember::Method(method) => {
                            let method_name = self.get_span_text(&method.name.span).to_lowercase();
                            if method_name == "__construct" {
                                for param in method.parameter_list.parameters.iter() {
                                    if param.is_promoted_property() {
                                        let prop_name = self.get_span_text(&param.variable.span);
                                        let name = prop_name.trim_start_matches('$').to_string();
                                        let name_lower = name.to_lowercase();

                                        let (type_hint, is_nullable) = self.extract_param_type(param);

                                        properties.insert(
                                            name_lower,
                                            PropertyInfo {
                                                name,
                                                type_hint,
                                                is_nullable,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

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

    fn extract_property_type(&self, prop: &Property<'_>) -> (Option<String>, bool) {
        match prop {
            Property::Plain(plain) => {
                if let Some(hint) = &plain.hint {
                    let type_str = self.get_span_text(&hint.span()).to_string();
                    let is_nullable = type_str.starts_with('?');
                    (Some(type_str.trim_start_matches('?').to_string()), is_nullable)
                } else {
                    (None, false)
                }
            }
            Property::Hooked(hooked) => {
                if let Some(hint) = &hooked.hint {
                    let type_str = self.get_span_text(&hint.span()).to_string();
                    let is_nullable = type_str.starts_with('?');
                    (Some(type_str.trim_start_matches('?').to_string()), is_nullable)
                } else {
                    (None, false)
                }
            }
        }
    }

    fn extract_param_type(&self, param: &FunctionLikeParameter<'_>) -> (Option<String>, bool) {
        if let Some(hint) = &param.hint {
            let type_str = self.get_span_text(&hint.span()).to_string();
            let is_nullable = type_str.starts_with('?');
            (Some(type_str.trim_start_matches('?').to_string()), is_nullable)
        } else {
            (None, false)
        }
    }

    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();
                self.current_class = Some(class_name.to_lowercase());

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
                self.analyze_expression(&expr_stmt.expression);
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::If(if_stmt) => {
                self.analyze_if_body(&if_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.analyze_foreach_body(&foreach.body);
            }
            Statement::For(for_stmt) => {
                self.analyze_for_body(&for_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.analyze_while_body(&while_stmt.body);
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

    fn analyze_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_expression<'a>(&mut self, expr: &Expression<'a>) {
        if let Expression::Assignment(assign) = expr {
            // Check for $this->property = value
            if let Expression::Access(Access::Property(prop_access)) = &*assign.lhs {
                // Only handle $this->property for now
                if let Expression::Variable(Variable::Direct(var)) = &*prop_access.object {
                    let var_name = self.get_span_text(&var.span);
                    if var_name == "$this" {
                        if let ClassLikeMemberSelector::Identifier(ident) = &prop_access.property {
                            let prop_name = self.get_span_text(&ident.span).to_lowercase();

                            // Clone data to avoid borrow checker issues
                            let prop_info_opt = self.current_class.as_ref().and_then(|class_name| {
                                self.class_properties.get(class_name).and_then(|properties| {
                                    properties.get(&prop_name).cloned()
                                })
                            });

                            if let Some(prop_info) = prop_info_opt {
                                if let Some(expected_type) = &prop_info.type_hint {
                                    self.check_assignment_type(
                                        assign.rhs,
                                        expected_type,
                                        prop_info.is_nullable,
                                        &prop_info.name,
                                        assign.lhs.span(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_assignment_type<'a>(
        &mut self,
        value: &Expression<'a>,
        expected_type: &str,
        is_nullable: bool,
        prop_name: &str,
        assignment_span: mago_span::Span,
    ) {
        let actual_type = self.infer_expression_type(value);

        if let Some(actual) = actual_type {
            let expected_lower = expected_type.to_lowercase();
            let actual_lower = actual.to_lowercase();

            // Check if types are compatible
            if self.types_compatible(&expected_lower, &actual_lower, is_nullable) {
                return;
            }

            let (line, col) = self.get_line_col(assignment_span.start.offset as usize);
            self.issues.push(
                Issue::error(
                    "property.type",
                    format!(
                        "Property ${} ({}{}) cannot be assigned {} value.",
                        prop_name,
                        if is_nullable { "?" } else { "" },
                        expected_type,
                        actual
                    ),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("property.typeMismatch"),
            );
        }
    }

    fn infer_expression_type<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::String(_) => Some("string".to_string()),
                Literal::Integer(_) => Some("int".to_string()),
                Literal::Float(_) => Some("float".to_string()),
                Literal::True(_) | Literal::False(_) => Some("bool".to_string()),
                Literal::Null(_) => Some("null".to_string()),
            },
            Expression::Array(_) | Expression::LegacyArray(_) => Some("array".to_string()),
            Expression::Instantiation(inst) => {
                if let Expression::Identifier(ident) = &*inst.class {
                    Some(self.get_span_text(&ident.span()).to_string())
                } else {
                    Some("object".to_string())
                }
            }
            Expression::Closure(_) | Expression::ArrowFunction(_) => Some("Closure".to_string()),
            _ => None,
        }
    }

    fn types_compatible(&self, expected: &str, actual: &str, is_nullable: bool) -> bool {
        if expected == actual {
            return true;
        }

        // mixed accepts everything
        if expected == "mixed" {
            return true;
        }

        // null is compatible with nullable types
        if actual == "null" && is_nullable {
            return true;
        }

        // int is compatible with float
        if expected == "float" && actual == "int" {
            return true;
        }

        // Scalar types
        if expected == "scalar" && matches!(actual, "int" | "float" | "string" | "bool") {
            return true;
        }

        // object type accepts any class
        if expected == "object" {
            return true;
        }

        // iterable accepts arrays
        if expected == "iterable" && actual == "array" {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_type_check_level() {
        let check = PropertyTypeCheck;
        assert_eq!(check.level(), 3);
    }
}
