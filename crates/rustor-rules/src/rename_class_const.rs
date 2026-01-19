//! Rule: rename_class_const (Configurable)
//!
//! Renames class constant references based on a configurable mapping.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.rename_class_const]
//! mappings = [
//!     { class = "SomeClass", old = "OLD_CONST", new = "NEW_CONST" },
//!     { class = "SomeClass", old = "OTHER_CONST", new_class = "DifferentClass", new = "NEW_CONST" }
//! ]
//! ```
//!
//! Pattern:
//! ```php
//! // Before
//! $value = SomeClass::OLD_CONST;
//!
//! // After
//! $value = SomeClass::NEW_CONST;
//! // or
//! $value = DifferentClass::NEW_CONST;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// A single class constant rename mapping
#[derive(Debug, Clone)]
pub struct ClassConstMapping {
    pub class: String,
    pub old_const: String,
    pub new_class: Option<String>,
    pub new_const: String,
}

/// Configuration for the rename_class_const rule
#[derive(Debug, Clone, Default)]
pub struct RenameClassConstConfig {
    pub mappings: Vec<ClassConstMapping>,
}

pub fn check_rename_class_const<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_rename_class_const_with_config(program, source, &RenameClassConstConfig::default())
}

pub fn check_rename_class_const_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &RenameClassConstConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut checker = RenameClassConstChecker {
        source,
        config,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct RenameClassConstChecker<'s, 'c> {
    source: &'s str,
    config: &'c RenameClassConstConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> RenameClassConstChecker<'s, 'c> {
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
                self.check_foreach_body(&foreach_stmt.body);
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

    fn check_expression(&mut self, expr: &Expression<'_>) {
        match expr {
            // ClassName::CONSTANT
            Expression::Access(Access::ClassConstant(access)) => {
                // Get class name - must be a static identifier
                if let Expression::Identifier(class_ident) = &*access.class {
                    let class_name = self.get_text(class_ident.span());

                    // Get constant name
                    let const_name = match &access.constant {
                        ClassLikeConstantSelector::Identifier(ident) => {
                            self.get_text(ident.span())
                        }
                        ClassLikeConstantSelector::Expression(_) => return, // Dynamic - skip
                    };

                    // Find mapping
                    for mapping in &self.config.mappings {
                        if self.matches_class(&mapping.class, class_name) && mapping.old_const == const_name {
                            let replacement = if let Some(ref new_class) = mapping.new_class {
                                format!("{}::{}", new_class, mapping.new_const)
                            } else {
                                format!("{}::{}", class_name, mapping.new_const)
                            };

                            self.edits.push(Edit::new(
                                expr.span(),
                                replacement,
                                format!("Rename class constant {}::{}", class_name, const_name),
                            ));
                            return;
                        }
                    }
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

    fn matches_class(&self, pattern: &str, actual: &str) -> bool {
        // Case-insensitive comparison for class names
        pattern.eq_ignore_ascii_case(actual)
            || pattern.rsplit('\\').next().map_or(false, |p| p.eq_ignore_ascii_case(actual))
    }
}

pub struct RenameClassConstRule {
    config: RenameClassConstConfig,
}

impl RenameClassConstRule {
    pub fn new() -> Self {
        Self {
            config: RenameClassConstConfig::default(),
        }
    }
}

impl Default for RenameClassConstRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RenameClassConstRule {
    fn name(&self) -> &'static str {
        "rename_class_const"
    }

    fn description(&self) -> &'static str {
        "Rename class constant references based on configurable mapping"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_rename_class_const_with_config(program, source, &self.config)
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
            description: "List of class constant rename mappings",
            default: "[]",
            option_type: ConfigOptionType::String,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for RenameClassConstRule {
    fn with_config(_config: &HashMap<String, ConfigValue>) -> Self {
        // Complex config parsing would go here
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &RenameClassConstConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_rename_class_const_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &RenameClassConstConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: Vec<ClassConstMapping>) -> RenameClassConstConfig {
        RenameClassConstConfig { mappings }
    }

    #[test]
    fn test_simple_rename() {
        let source = r#"<?php
$value = SomeClass::OLD_CONST;
"#;
        let config = make_config(vec![ClassConstMapping {
            class: "SomeClass".to_string(),
            old_const: "OLD_CONST".to_string(),
            new_class: None,
            new_const: "NEW_CONST".to_string(),
        }]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("SomeClass::NEW_CONST"));
    }

    #[test]
    fn test_rename_with_new_class() {
        let source = r#"<?php
$value = SomeClass::OLD_CONST;
"#;
        let config = make_config(vec![ClassConstMapping {
            class: "SomeClass".to_string(),
            old_const: "OLD_CONST".to_string(),
            new_class: Some("DifferentClass".to_string()),
            new_const: "NEW_CONST".to_string(),
        }]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("DifferentClass::NEW_CONST"));
    }

    #[test]
    fn test_multiple_renames() {
        let source = r#"<?php
$a = SomeClass::OLD_CONST;
$b = SomeClass::OTHER_CONST;
"#;
        let config = make_config(vec![
            ClassConstMapping {
                class: "SomeClass".to_string(),
                old_const: "OLD_CONST".to_string(),
                new_class: None,
                new_const: "NEW_CONST".to_string(),
            },
            ClassConstMapping {
                class: "SomeClass".to_string(),
                old_const: "OTHER_CONST".to_string(),
                new_class: None,
                new_const: "ANOTHER_CONST".to_string(),
            },
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_unmatched_class() {
        let source = r#"<?php
$value = OtherClass::OLD_CONST;
"#;
        let config = make_config(vec![ClassConstMapping {
            class: "SomeClass".to_string(),
            old_const: "OLD_CONST".to_string(),
            new_class: None,
            new_const: "NEW_CONST".to_string(),
        }]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_unmatched_const() {
        let source = r#"<?php
$value = SomeClass::DIFFERENT_CONST;
"#;
        let config = make_config(vec![ClassConstMapping {
            class: "SomeClass".to_string(),
            old_const: "OLD_CONST".to_string(),
            new_class: None,
            new_const: "NEW_CONST".to_string(),
        }]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$value = SomeClass::CONST;
"#;
        let config = RenameClassConstConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }
}
