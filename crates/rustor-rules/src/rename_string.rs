//! Rule: rename_string (Configurable)
//!
//! Renames string literal values based on a configurable mapping.
//!
//! Pattern:
//! ```php
//! // Before
//! return 'ROLE_PREVIOUS_ADMIN';
//!
//! // After
//! return 'IS_IMPERSONATOR';
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// Configuration for the rename_string rule
#[derive(Debug, Clone, Default)]
pub struct RenameStringConfig {
    /// Map of old string values to new string values
    pub mappings: HashMap<String, String>,
}

pub fn check_rename_string<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_rename_string_with_config(program, source, &RenameStringConfig::default())
}

pub fn check_rename_string_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &RenameStringConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut checker = RenameStringChecker {
        source,
        config,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct RenameStringChecker<'s, 'c> {
    source: &'s str,
    config: &'c RenameStringConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> RenameStringChecker<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn check_program(&mut self, program: &Program<'_>) {
        for stmt in program.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::Function(func) => {
                self.check_block(&func.body);
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(ref body) = method.body {
                            self.check_block(body);
                        }
                    }
                }
            }
            Statement::Namespace(ns) => {
                let statements = match &ns.body {
                    NamespaceBody::Implicit(body) => &body.statements,
                    NamespaceBody::BraceDelimited(body) => &body.statements,
                };
                for inner in statements.iter() {
                    self.check_statement(inner);
                }
            }
            Statement::Block(block) => {
                self.check_block(block);
            }
            Statement::If(if_stmt) => {
                self.check_expression(&if_stmt.condition);
                self.check_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.check_expression(&while_stmt.condition);
                self.check_while_body(&while_stmt.body);
            }
            Statement::For(for_stmt) => {
                self.check_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach_stmt) => {
                self.check_expression(&foreach_stmt.expression);
                self.check_foreach_body(&foreach_stmt.body);
            }
            Statement::Switch(switch_stmt) => {
                self.check_expression(&switch_stmt.expression);
                self.check_switch_body(&switch_stmt.body);
            }
            Statement::Try(try_stmt) => {
                self.check_block(&try_stmt.block);
                for catch in try_stmt.catch_clauses.iter() {
                    self.check_block(&catch.block);
                }
                if let Some(ref finally) = try_stmt.finally_clause {
                    self.check_block(&finally.block);
                }
            }
            Statement::Expression(expr_stmt) => {
                self.check_expression(&expr_stmt.expression);
            }
            Statement::Return(ret) => {
                if let Some(ref val) = ret.value {
                    self.check_expression(val);
                }
            }
            Statement::Echo(echo_stmt) => {
                for val in echo_stmt.values.iter() {
                    self.check_expression(val);
                }
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &Block<'_>) {
        for stmt in block.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_if_body(&mut self, body: &IfBody<'_>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.check_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                    self.check_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.check_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_while_body(&mut self, body: &WhileBody<'_>) {
        match body {
            WhileBody::Statement(stmt) => self.check_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_for_body(&mut self, body: &ForBody<'_>) {
        match body {
            ForBody::Statement(stmt) => self.check_statement(stmt),
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_foreach_body(&mut self, body: &ForeachBody<'_>) {
        match body {
            ForeachBody::Statement(stmt) => self.check_statement(stmt),
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_switch_body(&mut self, body: &SwitchBody<'_>) {
        let cases = match body {
            SwitchBody::BraceDelimited(block) => &block.cases,
            SwitchBody::ColonDelimited(block) => &block.cases,
        };
        for case in cases.iter() {
            match case {
                SwitchCase::Expression(expr_case) => {
                    self.check_expression(&expr_case.expression);
                    for inner in expr_case.statements.iter() {
                        self.check_statement(inner);
                    }
                }
                SwitchCase::Default(default_case) => {
                    for inner in default_case.statements.iter() {
                        self.check_statement(inner);
                    }
                }
            }
        }
    }

    fn check_expression(&mut self, expr: &Expression<'_>) {
        match expr {
            // Check string literals
            Expression::Literal(Literal::String(string_lit)) => {
                let full_text = self.get_text(string_lit.span());

                // Extract the string value (remove quotes)
                let quote_char = full_text.chars().next().unwrap_or('"');
                if quote_char != '\'' && quote_char != '"' {
                    return; // Heredoc or nowdoc - skip for simplicity
                }

                let string_value = &full_text[1..full_text.len() - 1];

                if let Some(new_value) = self.config.mappings.get(string_value) {
                    let replacement = format!("{}{}{}", quote_char, new_value, quote_char);
                    self.edits.push(Edit::new(
                        string_lit.span(),
                        replacement,
                        format!("Rename string '{}' to '{}'", string_value, new_value),
                    ));
                }
            }
            // Recursive traversal
            Expression::Binary(binary) => {
                self.check_expression(&binary.lhs);
                self.check_expression(&binary.rhs);
            }
            Expression::UnaryPrefix(unary) => {
                self.check_expression(&unary.operand);
            }
            Expression::UnaryPostfix(unary) => {
                self.check_expression(&unary.operand);
            }
            Expression::Parenthesized(paren) => {
                self.check_expression(&paren.expression);
            }
            Expression::Conditional(cond) => {
                self.check_expression(&cond.condition);
                if let Some(ref then_expr) = cond.then {
                    self.check_expression(then_expr);
                }
                self.check_expression(&cond.r#else);
            }
            Expression::Assignment(assign) => {
                self.check_expression(&assign.lhs);
                self.check_expression(&assign.rhs);
            }
            Expression::Call(call) => {
                match call {
                    Call::Function(func_call) => {
                        for arg in func_call.argument_list.arguments.iter() {
                            let arg_expr = match arg {
                                Argument::Positional(pos) => &pos.value,
                                Argument::Named(named) => &named.value,
                            };
                            self.check_expression(arg_expr);
                        }
                    }
                    Call::Method(method_call) => {
                        self.check_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            let arg_expr = match arg {
                                Argument::Positional(pos) => &pos.value,
                                Argument::Named(named) => &named.value,
                            };
                            self.check_expression(arg_expr);
                        }
                    }
                    Call::StaticMethod(static_call) => {
                        for arg in static_call.argument_list.arguments.iter() {
                            let arg_expr = match arg {
                                Argument::Positional(pos) => &pos.value,
                                Argument::Named(named) => &named.value,
                            };
                            self.check_expression(arg_expr);
                        }
                    }
                    Call::NullSafeMethod(ns_call) => {
                        self.check_expression(&ns_call.object);
                        for arg in ns_call.argument_list.arguments.iter() {
                            let arg_expr = match arg {
                                Argument::Positional(pos) => &pos.value,
                                Argument::Named(named) => &named.value,
                            };
                            self.check_expression(arg_expr);
                        }
                    }
                }
            }
            Expression::Array(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.check_expression(&kv.key);
                            self.check_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.check_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.check_expression(&var.value);
                        }
                        _ => {}
                    }
                }
            }
            Expression::Closure(closure) => {
                self.check_block(&closure.body);
            }
            Expression::ArrowFunction(arrow) => {
                self.check_expression(&arrow.expression);
            }
            _ => {}
        }
    }
}

pub struct RenameStringRule {
    config: RenameStringConfig,
}

impl RenameStringRule {
    pub fn new() -> Self {
        Self {
            config: RenameStringConfig::default(),
        }
    }

    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        Self {
            config: RenameStringConfig { mappings },
        }
    }
}

impl Default for RenameStringRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RenameStringRule {
    fn name(&self) -> &'static str {
        "rename_string"
    }

    fn description(&self) -> &'static str {
        "Rename string literal values based on configurable mapping"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_rename_string_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[ConfigOption {
            name: "mappings",
            description: "Map of old string values to new string values",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for RenameStringRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let mappings = config
            .get("mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        Self {
            config: RenameStringConfig { mappings },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &RenameStringConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_rename_string_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &RenameStringConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str)]) -> RenameStringConfig {
        RenameStringConfig {
            mappings: mappings
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn test_simple_string() {
        let source = r#"<?php
$role = 'ROLE_PREVIOUS_ADMIN';
"#;
        let config = make_config(&[("ROLE_PREVIOUS_ADMIN", "IS_IMPERSONATOR")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("'IS_IMPERSONATOR'"));
    }

    #[test]
    fn test_double_quoted_string() {
        let source = r#"<?php
$role = "ROLE_PREVIOUS_ADMIN";
"#;
        let config = make_config(&[("ROLE_PREVIOUS_ADMIN", "IS_IMPERSONATOR")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("\"IS_IMPERSONATOR\""));
    }

    #[test]
    fn test_string_in_function_call() {
        let source = r#"<?php
hasRole('ROLE_PREVIOUS_ADMIN');
"#;
        let config = make_config(&[("ROLE_PREVIOUS_ADMIN", "IS_IMPERSONATOR")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_string_in_array() {
        let source = r#"<?php
$roles = ['ROLE_PREVIOUS_ADMIN', 'ROLE_USER'];
"#;
        let config = make_config(&[("ROLE_PREVIOUS_ADMIN", "IS_IMPERSONATOR")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_string_in_return() {
        let source = r#"<?php
function getRole() {
    return 'ROLE_PREVIOUS_ADMIN';
}
"#;
        let config = make_config(&[("ROLE_PREVIOUS_ADMIN", "IS_IMPERSONATOR")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_strings() {
        let source = r#"<?php
$a = 'old_value';
$b = 'another_old';
"#;
        let config = make_config(&[("old_value", "new_value"), ("another_old", "another_new")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$role = 'SOME_OTHER_ROLE';
"#;
        let config = make_config(&[("ROLE_PREVIOUS_ADMIN", "IS_IMPERSONATOR")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$role = 'ROLE_PREVIOUS_ADMIN';
"#;
        let config = RenameStringConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_preserves_quote_style() {
        let source = r#"<?php
$a = 'single';
$b = "double";
"#;
        let config = make_config(&[("single", "new_single"), ("double", "new_double")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("'new_single'"));
        assert!(result.contains("\"new_double\""));
    }
}
