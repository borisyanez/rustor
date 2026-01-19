//! Rule: split_double_assign
//!
//! Splits chained assignments into separate statements.
//!
//! Pattern:
//! ```php
//! // Before
//! $a = $b = 1;
//!
//! // After
//! $b = 1;
//! $a = 1;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_split_double_assign<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut checker = SplitDoubleAssignChecker {
        source,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct SplitDoubleAssignChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SplitDoubleAssignChecker<'s> {
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
                // Check for chained assignment: $a = $b = value
                if let Expression::Assignment(assign) = &expr_stmt.expression {
                    if let Expression::Assignment(inner_assign) = &*assign.rhs {
                        // We have a chained assignment
                        self.process_chained_assignment(assign, &expr_stmt.expression);
                    }
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
                self.check_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
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

    fn process_chained_assignment(&mut self, assign: &Assignment<'_>, full_expr: &Expression<'_>) {
        // Collect all variables and find the final value
        let mut vars = Vec::new();
        let mut current: &Expression<'_> = &*assign.rhs;

        // First variable is from the outer assignment
        vars.push(self.get_text(assign.lhs.span()));

        // Walk through the chain
        while let Expression::Assignment(inner) = current {
            vars.push(self.get_text(inner.lhs.span()));
            current = &*inner.rhs;
        }

        // `current` is now the final value
        let value = self.get_text(current.span());

        // Skip if any variable is an array access (could have side effects)
        for var in &vars {
            if var.contains('[') {
                return;
            }
        }

        // Build the replacement: assignments in reverse order (innermost first)
        let mut assignments = Vec::new();
        for var in vars.iter().rev() {
            assignments.push(format!("{} = {}", var, value));
        }

        let replacement = assignments.join(";\n");

        self.edits.push(Edit::new(
            full_expr.span(),
            replacement,
            "Split chained assignment into separate statements".to_string(),
        ));
    }
}

pub struct SplitDoubleAssignRule;

impl SplitDoubleAssignRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SplitDoubleAssignRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for SplitDoubleAssignRule {
    fn name(&self) -> &'static str {
        "split_double_assign"
    }

    fn description(&self) -> &'static str {
        "Split chained assignments into separate statements"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_split_double_assign(program, source)
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
        check_split_double_assign(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_simple_double_assign() {
        let source = r#"<?php
$a = $b = 1;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$b = 1"));
        assert!(result.contains("$a = 1"));
    }

    #[test]
    fn test_triple_assign() {
        let source = r#"<?php
$a = $b = $c = 'value';
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$c = 'value'"));
        assert!(result.contains("$b = 'value'"));
        assert!(result.contains("$a = 'value'"));
    }

    #[test]
    fn test_skip_single_assign() {
        let source = r#"<?php
$a = 1;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_in_function() {
        let source = r#"<?php
function test() {
    $x = $y = 0;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_array_access() {
        // Array access could have side effects, skip
        let source = r#"<?php
$arr[$i] = $b = 1;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }
}
