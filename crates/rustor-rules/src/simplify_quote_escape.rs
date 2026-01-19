//! Rule: simplify_quote_escape
//!
//! Switches quote style to avoid escaped quotes inside strings.
//!
//! Pattern:
//! ```php
//! // Before
//! $name = "\" Tom";
//! $name = '\' Sara';
//!
//! // After
//! $name = '" Tom';
//! $name = "' Sara";
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_simplify_quote_escape<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut checker = SimplifyQuoteEscapeChecker {
        source,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct SimplifyQuoteEscapeChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SimplifyQuoteEscapeChecker<'s> {
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
            Statement::Expression(expr_stmt) => {
                self.check_expression(&expr_stmt.expression);
            }
            Statement::Return(ret) => {
                if let Some(ref val) = ret.value {
                    self.check_expression(val);
                }
            }
            Statement::Echo(echo) => {
                for val in echo.values.iter() {
                    self.check_expression(val);
                }
            }
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
            Expression::Literal(Literal::String(string_lit)) => {
                self.check_string_literal(string_lit);
            }
            Expression::Binary(binary) => {
                self.check_expression(&binary.lhs);
                self.check_expression(&binary.rhs);
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
            Expression::Conditional(cond) => {
                self.check_expression(&cond.condition);
                if let Some(ref then_expr) = cond.then {
                    self.check_expression(then_expr);
                }
                self.check_expression(&cond.r#else);
            }
            _ => {}
        }
    }

    fn check_string_literal(&mut self, string_lit: &LiteralString<'_>) {
        let span = string_lit.span();
        let full_text = self.get_text(span);

        // Get quote character
        let quote_char = match full_text.chars().next() {
            Some('\'') => '\'',
            Some('"') => '"',
            _ => return, // Heredoc/nowdoc - skip
        };

        // Get the string content (without quotes)
        if full_text.len() < 2 {
            return;
        }
        let content = full_text[1..full_text.len() - 1].to_string();

        // Count quotes in the content
        let single_quotes = content.matches('\'').count();
        let double_quotes = content.matches('"').count();
        let escaped_single = content.matches("\\'").count();
        let escaped_double = content.matches("\\\"").count();

        // Check for special characters that need double quotes
        let has_special = content.contains('\\') &&
            (content.contains("\\n") || content.contains("\\t") ||
             content.contains("\\r") || content.contains("\\$") ||
             content.contains("\\\\"));

        // Skip if content has variables (needs double quotes)
        if content.contains('$') && quote_char == '"' {
            return;
        }

        // For single-quoted strings: switch to double if it has escaped single quotes
        // and no double quotes
        if quote_char == '\'' && escaped_single > 0 && double_quotes == 0 && !has_special {
            let new_content = content.replace("\\'", "'");
            let replacement = format!("\"{}\"", new_content);
            self.edits.push(Edit::new(
                span,
                replacement,
                "Switch to double quotes to avoid escaping".to_string(),
            ));
        }

        // For double-quoted strings: switch to single if it has escaped double quotes
        // and no single quotes, and no special escape sequences
        if quote_char == '"' && escaped_double > 0 && single_quotes == 0 && !has_special {
            // Check for other escape sequences that only work in double quotes
            if !content.contains("\\n") && !content.contains("\\t") &&
               !content.contains("\\r") && !content.contains("\\$") &&
               !content.contains('$') {
                let new_content = content.replace("\\\"", "\"");
                let replacement = format!("'{}'", new_content);
                self.edits.push(Edit::new(
                    span,
                    replacement,
                    "Switch to single quotes to avoid escaping".to_string(),
                ));
            }
        }
    }
}

pub struct SimplifyQuoteEscapeRule;

impl SimplifyQuoteEscapeRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimplifyQuoteEscapeRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for SimplifyQuoteEscapeRule {
    fn name(&self) -> &'static str {
        "simplify_quote_escape"
    }

    fn description(&self) -> &'static str {
        "Switch quote style to avoid escaped quotes inside strings"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_quote_escape(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_simplify_quote_escape(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_escaped_single_to_double() {
        let source = r#"<?php
$name = 'It\'s a test';
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("\"It's a test\""));
    }

    #[test]
    fn test_escaped_double_to_single() {
        let source = r#"<?php
$name = "He said \"hello\"";
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("'He said \"hello\"'"));
    }

    #[test]
    fn test_skip_no_escapes() {
        let source = r#"<?php
$name = 'hello';
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_both_quotes() {
        // If string has both types, can't simplify
        let source = r#"<?php
$name = "It's \"complex\"";
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_variables() {
        // Double-quoted strings with variables must stay double-quoted
        let source = r#"<?php
$name = "Hello \"$user\"";
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_escape_sequences() {
        // Strings with \n, \t etc must stay double-quoted
        let source = r#"<?php
$name = "line1\nline2";
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }
}
