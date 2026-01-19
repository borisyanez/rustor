//! Rule: post_to_pre_increment
//!
//! Converts standalone post-increment/decrement to pre-increment/decrement.
//! Pre-increment is slightly faster as it doesn't need to store the original value.
//!
//! Pattern:
//! ```php
//! // Before
//! $i++;
//! $i--;
//! for ($i = 0; $i < 10; $i++) {}
//!
//! // After
//! ++$i;
//! --$i;
//! for ($i = 0; $i < 10; ++$i) {}
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_post_to_pre_increment<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut checker = PostToPreIncrementChecker {
        source,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct PostToPreIncrementChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> PostToPreIncrementChecker<'s> {
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
            // Standalone expression statement: $i++;
            Statement::Expression(expr_stmt) => {
                self.check_standalone_expression(&expr_stmt.expression);
                // Also recurse into expression for nested structures
                self.check_expression_recursive(&expr_stmt.expression);
            }
            // For loop: for ($i = 0; $i < 10; $i++)
            Statement::For(for_stmt) => {
                // Check the loop increment expressions
                for expr in for_stmt.increments.iter() {
                    self.check_standalone_expression(expr);
                }
                self.check_for_body(&for_stmt.body);
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
                self.check_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.check_while_body(&while_stmt.body);
            }
            Statement::Foreach(foreach_stmt) => {
                self.check_foreach_body(&foreach_stmt.body);
            }
            Statement::Switch(switch_stmt) => {
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

    fn check_switch_body(&mut self, body: &SwitchBody<'_>) {
        let cases = match body {
            SwitchBody::BraceDelimited(block) => &block.cases,
            SwitchBody::ColonDelimited(block) => &block.cases,
        };
        for case in cases.iter() {
            match case {
                SwitchCase::Expression(expr_case) => {
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

    /// Check if expression is a standalone post-increment/decrement
    fn check_standalone_expression(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::UnaryPostfix(postfix) => {
                let operator = self.get_text(postfix.operator.span());
                let operand = self.get_text(postfix.operand.span());

                if operator == "++" {
                    self.edits.push(Edit::new(
                        expr.span(),
                        format!("++{}", operand),
                        format!("Convert {}++ to ++{}", operand, operand),
                    ));
                } else if operator == "--" {
                    self.edits.push(Edit::new(
                        expr.span(),
                        format!("--{}", operand),
                        format!("Convert {}-- to --{}", operand, operand),
                    ));
                }
            }
            _ => {}
        }
    }

    /// Recursively check expressions for nested closures/functions
    fn check_expression_recursive(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::Closure(closure) => {
                self.check_block(&closure.body);
            }
            Expression::ArrowFunction(arrow) => {
                self.check_expression_recursive(&arrow.expression);
            }
            _ => {}
        }
    }
}

pub struct PostToPreIncrementRule;

impl PostToPreIncrementRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PostToPreIncrementRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for PostToPreIncrementRule {
    fn name(&self) -> &'static str {
        "post_to_pre_increment"
    }

    fn description(&self) -> &'static str {
        "Convert standalone post-increment/decrement to pre-increment/decrement"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_post_to_pre_increment(program, source)
    }

    fn category(&self) -> Category {
        Category::Performance
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
        check_post_to_pre_increment(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_standalone_post_increment() {
        let source = r#"<?php
$i++;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("++$i"));
    }

    #[test]
    fn test_standalone_post_decrement() {
        let source = r#"<?php
$i--;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("--$i"));
    }

    #[test]
    fn test_for_loop_increment() {
        let source = r#"<?php
for ($i = 0; $i < 10; $i++) {
    echo $i;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("++$i"));
    }

    #[test]
    fn test_for_loop_decrement() {
        let source = r#"<?php
for ($i = 10; $i > 0; $i--) {
    echo $i;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("--$i"));
    }

    #[test]
    fn test_in_function() {
        let source = r#"<?php
function test() {
    $count++;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_pre_increment() {
        let source = r#"<?php
++$i;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_expression_context() {
        // When used in expression context, we should NOT change it
        // because $a = $i++ has different semantics than $a = ++$i
        // This rule only changes STANDALONE statements
        let source = r#"<?php
$a = $i++;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_increments() {
        let source = r#"<?php
$a++;
$b--;
$c++;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 3);
    }
}
