//! Rule: rename_constant (Configurable)
//!
//! Renames global constant references based on a configurable mapping.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.rename_constant]
//! mappings = { "MYSQL_ASSOC" = "MYSQLI_ASSOC", "MYSQL_NUM" = "MYSQLI_NUM" }
//! ```
//!
//! Pattern:
//! ```php
//! // Before
//! $value = MYSQL_ASSOC;
//!
//! // After
//! $value = MYSQLI_ASSOC;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// Configuration for the rename_constant rule
#[derive(Debug, Clone, Default)]
pub struct RenameConstantConfig {
    /// Map of old constant names to new constant names
    pub mappings: HashMap<String, String>,
}

/// Check a parsed PHP program for constant references to rename
pub fn check_rename_constant<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_rename_constant_with_config(program, source, &RenameConstantConfig::default())
}

/// Check a parsed PHP program for constant references to rename with configuration
pub fn check_rename_constant_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &RenameConstantConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut checker = RenameConstantChecker {
        source,
        config,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct RenameConstantChecker<'s, 'c> {
    source: &'s str,
    config: &'c RenameConstantConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> RenameConstantChecker<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn find_mapping(&self, const_name: &str) -> Option<&String> {
        self.config.mappings.get(const_name)
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
                for init in for_stmt.initializations.iter() {
                    self.check_expression(init);
                }
                for cond in for_stmt.conditions.iter() {
                    self.check_expression(cond);
                }
                for inc in for_stmt.increments.iter() {
                    self.check_expression(inc);
                }
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
            // Check for constant identifiers: CONSTANT_NAME
            Expression::Identifier(ident) => {
                let const_name = self.get_text(ident.span());

                // Skip built-in constants and lowercase identifiers (likely function names)
                let lower = const_name.to_lowercase();
                if matches!(lower.as_str(), "true" | "false" | "null") {
                    return;
                }

                if let Some(new_name) = self.find_mapping(const_name) {
                    self.edits.push(Edit::new(
                        ident.span(),
                        new_name.clone(),
                        format!("Rename constant {} to {}", const_name, new_name),
                    ));
                }
            }
            // Also check ConstantAccess for namespaced constants
            Expression::ConstantAccess(access) => {
                let const_name = self.get_text(access.span());

                if let Some(new_name) = self.find_mapping(const_name) {
                    self.edits.push(Edit::new(
                        access.span(),
                        new_name.clone(),
                        format!("Rename constant {} to {}", const_name, new_name),
                    ));
                }
            }
            // Recursive traversal for other expressions
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

/// Rule to rename global constant references based on configuration
pub struct RenameConstantRule {
    config: RenameConstantConfig,
}

impl RenameConstantRule {
    pub fn new() -> Self {
        Self {
            config: RenameConstantConfig::default(),
        }
    }

    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        Self {
            config: RenameConstantConfig { mappings },
        }
    }
}

impl Default for RenameConstantRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RenameConstantRule {
    fn name(&self) -> &'static str {
        "rename_constant"
    }

    fn description(&self) -> &'static str {
        "Rename global constant references based on configurable mapping"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_rename_constant_with_config(program, source, &self.config)
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
            description: "Map of old constant names to new constant names",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for RenameConstantRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let mappings = config
            .get("mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        Self {
            config: RenameConstantConfig { mappings },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &RenameConstantConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_rename_constant_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &RenameConstantConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str)]) -> RenameConstantConfig {
        RenameConstantConfig {
            mappings: mappings
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn test_simple_constant() {
        let source = r#"<?php
$value = MYSQL_ASSOC;
"#;
        let config = make_config(&[("MYSQL_ASSOC", "MYSQLI_ASSOC")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("MYSQLI_ASSOC"));
    }

    #[test]
    fn test_multiple_constants() {
        let source = r#"<?php
$a = MYSQL_ASSOC;
$b = MYSQL_NUM;
"#;
        let config = make_config(&[("MYSQL_ASSOC", "MYSQLI_ASSOC"), ("MYSQL_NUM", "MYSQLI_NUM")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_constant_in_function_call() {
        let source = r#"<?php
mysqli_fetch_array($result, MYSQL_ASSOC);
"#;
        let config = make_config(&[("MYSQL_ASSOC", "MYSQLI_ASSOC")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("MYSQLI_ASSOC"));
    }

    #[test]
    fn test_constant_in_condition() {
        let source = r#"<?php
if ($mode === MYSQL_ASSOC) {
    doSomething();
}
"#;
        let config = make_config(&[("MYSQL_ASSOC", "MYSQLI_ASSOC")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_constant_in_switch() {
        let source = r#"<?php
switch ($mode) {
    case MYSQL_ASSOC:
        break;
    case MYSQL_NUM:
        break;
}
"#;
        let config = make_config(&[("MYSQL_ASSOC", "MYSQLI_ASSOC"), ("MYSQL_NUM", "MYSQLI_NUM")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_true_false_null() {
        let source = r#"<?php
$a = true;
$b = false;
$c = null;
"#;
        let config = make_config(&[("true", "TRUE"), ("false", "FALSE"), ("null", "NULL")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty(), "Should not rename built-in constants");
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$value = SOME_OTHER_CONST;
"#;
        let config = make_config(&[("MYSQL_ASSOC", "MYSQLI_ASSOC")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$value = MYSQL_ASSOC;
"#;
        let config = RenameConstantConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }
}
