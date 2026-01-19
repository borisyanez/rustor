//! Rule: rename_static_method (Configurable)
//!
//! Renames static method calls based on a configurable mapping.
//!
//! Pattern:
//! ```php
//! // Before
//! SomeClass::oldMethod();
//!
//! // After
//! AnotherClass::newMethod();
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// A single static method rename mapping
#[derive(Debug, Clone)]
pub struct StaticMethodMapping {
    pub class: String,
    pub old_method: String,
    pub new_class: Option<String>,
    pub new_method: String,
}

/// Configuration for the rename_static_method rule
#[derive(Debug, Clone, Default)]
pub struct RenameStaticMethodConfig {
    pub mappings: Vec<StaticMethodMapping>,
}

pub fn check_rename_static_method<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_rename_static_method_with_config(program, source, &RenameStaticMethodConfig::default())
}

pub fn check_rename_static_method_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &RenameStaticMethodConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut checker = RenameStaticMethodChecker {
        source,
        config,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct RenameStaticMethodChecker<'s, 'c> {
    source: &'s str,
    config: &'c RenameStaticMethodConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> RenameStaticMethodChecker<'s, 'c> {
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
            // ClassName::methodName()
            Expression::Call(Call::StaticMethod(static_call)) => {
                // Get class name
                if let Expression::Identifier(class_ident) = &*static_call.class {
                    let class_name = self.get_text(class_ident.span());

                    // Get method name
                    let method_name = match &static_call.method {
                        ClassLikeMemberSelector::Identifier(ident) => {
                            self.get_text(ident.span())
                        }
                        ClassLikeMemberSelector::Variable(_) => return, // Dynamic - skip
                        ClassLikeMemberSelector::Expression(_) => return, // Dynamic - skip
                    };

                    // Find mapping
                    for mapping in &self.config.mappings {
                        if self.matches_class(&mapping.class, class_name)
                            && mapping.old_method.eq_ignore_ascii_case(method_name)
                        {
                            // Build the arguments part
                            let args_text = self.get_text(static_call.argument_list.span());

                            let replacement = if let Some(ref new_class) = mapping.new_class {
                                format!("{}::{}{}", new_class, mapping.new_method, args_text)
                            } else {
                                format!("{}::{}{}", class_name, mapping.new_method, args_text)
                            };

                            self.edits.push(Edit::new(
                                expr.span(),
                                replacement,
                                format!("Rename static method {}::{}", class_name, method_name),
                            ));
                            return;
                        }
                    }
                }

                // Still check arguments
                for arg in static_call.argument_list.arguments.iter() {
                    let arg_expr = match arg {
                        Argument::Positional(pos) => &pos.value,
                        Argument::Named(named) => &named.value,
                    };
                    self.check_expression(arg_expr);
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
                    _ => {}
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
        pattern.eq_ignore_ascii_case(actual)
            || pattern.rsplit('\\').next().map_or(false, |p| p.eq_ignore_ascii_case(actual))
    }
}

pub struct RenameStaticMethodRule {
    config: RenameStaticMethodConfig,
}

impl RenameStaticMethodRule {
    pub fn new() -> Self {
        Self {
            config: RenameStaticMethodConfig::default(),
        }
    }
}

impl Default for RenameStaticMethodRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RenameStaticMethodRule {
    fn name(&self) -> &'static str {
        "rename_static_method"
    }

    fn description(&self) -> &'static str {
        "Rename static method calls based on configurable mapping"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_rename_static_method_with_config(program, source, &self.config)
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
            description: "List of static method rename mappings",
            default: "[]",
            option_type: ConfigOptionType::String,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for RenameStaticMethodRule {
    fn with_config(_config: &HashMap<String, ConfigValue>) -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &RenameStaticMethodConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_rename_static_method_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &RenameStaticMethodConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: Vec<StaticMethodMapping>) -> RenameStaticMethodConfig {
        RenameStaticMethodConfig { mappings }
    }

    #[test]
    fn test_simple_rename() {
        let source = r#"<?php
$result = SomeClass::oldMethod();
"#;
        let config = make_config(vec![StaticMethodMapping {
            class: "SomeClass".to_string(),
            old_method: "oldMethod".to_string(),
            new_class: None,
            new_method: "newMethod".to_string(),
        }]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("SomeClass::newMethod()"));
    }

    #[test]
    fn test_rename_with_new_class() {
        let source = r#"<?php
$result = SomeClass::oldMethod();
"#;
        let config = make_config(vec![StaticMethodMapping {
            class: "SomeClass".to_string(),
            old_method: "oldMethod".to_string(),
            new_class: Some("AnotherClass".to_string()),
            new_method: "newMethod".to_string(),
        }]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("AnotherClass::newMethod()"));
    }

    #[test]
    fn test_with_arguments() {
        let source = r#"<?php
$result = SomeClass::oldMethod($a, $b);
"#;
        let config = make_config(vec![StaticMethodMapping {
            class: "SomeClass".to_string(),
            old_method: "oldMethod".to_string(),
            new_class: None,
            new_method: "newMethod".to_string(),
        }]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("SomeClass::newMethod($a, $b)"));
    }

    #[test]
    fn test_skip_unmatched_class() {
        let source = r#"<?php
$result = OtherClass::oldMethod();
"#;
        let config = make_config(vec![StaticMethodMapping {
            class: "SomeClass".to_string(),
            old_method: "oldMethod".to_string(),
            new_class: None,
            new_method: "newMethod".to_string(),
        }]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_unmatched_method() {
        let source = r#"<?php
$result = SomeClass::differentMethod();
"#;
        let config = make_config(vec![StaticMethodMapping {
            class: "SomeClass".to_string(),
            old_method: "oldMethod".to_string(),
            new_class: None,
            new_method: "newMethod".to_string(),
        }]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$result = SomeClass::method();
"#;
        let config = RenameStaticMethodConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }
}
