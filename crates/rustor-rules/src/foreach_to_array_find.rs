//! Rule: Convert foreach loops to array_find() (PHP 8.4+)
//!
//! Converts foreach loops that find the first matching element to array_find().
//!
//! Pattern:
//! ```php
//! // Before
//! $found = null;
//! foreach ($animals as $animal) {
//!     if (str_starts_with($animal, 'c')) {
//!         $found = $animal;
//!         break;
//!     }
//! }
//!
//! // After
//! $found = array_find($animals, fn($animal) => str_starts_with($animal, 'c'));
//! ```

use mago_span::{HasSpan, Span};
use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for foreach to array_find conversions
pub fn check_foreach_to_array_find<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut checker = ForeachToArrayFindChecker {
        source,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct ForeachToArrayFindChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> ForeachToArrayFindChecker<'s> {
    fn get_text(&self, span: Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn check_program(&mut self, program: &Program<'_>) {
        self.check_statement_sequence(program.statements.as_slice());
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
                self.check_statement_sequence(statements.as_slice());
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
                if let Some(finally) = &try_stmt.finally_clause {
                    self.check_block(&finally.block);
                }
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &Block<'_>) {
        self.check_statement_sequence(block.statements.as_slice());
    }

    fn check_statement_sequence(&mut self, statements: &[Statement<'_>]) {
        for stmt in statements.iter() {
            self.check_statement(stmt);
        }

        for i in 0..statements.len() {
            // Pattern: $found = null; foreach ($arr as $item) { if (cond) { $found = $item; break; } }
            if i > 0 {
                if let Some(edit) = self.check_null_assignment_pattern(&statements[i - 1], &statements[i]) {
                    self.edits.push(edit);
                }
            }
        }
    }

    fn check_null_assignment_pattern(
        &self,
        prev_stmt: &Statement<'_>,
        foreach_stmt: &Statement<'_>,
    ) -> Option<Edit> {
        let Statement::Expression(prev_expr_stmt) = prev_stmt else {
            return None;
        };
        let Expression::Assignment(prev_assign) = prev_expr_stmt.expression else {
            return None;
        };
        if !matches!(prev_assign.operator, AssignmentOperator::Assign(_)) {
            return None;
        }
        // Must assign null
        if !self.is_null_literal(&prev_assign.rhs) {
            return None;
        }
        let var_name = self.get_simple_variable_name(&prev_assign.lhs)?;

        let Statement::Foreach(foreach) = foreach_stmt else {
            return None;
        };

        let body_stmts = foreach.body.statements();
        if body_stmts.len() != 1 {
            return None;
        }

        let first_stmt = &body_stmts[0];
        let Statement::Block(block) = first_stmt else {
            return self.check_find_foreach_body(first_stmt, foreach, &var_name, prev_stmt);
        };

        let block_stmts = block.statements.as_slice();
        if block_stmts.len() != 1 {
            return None;
        }

        self.check_find_foreach_body(&block_stmts[0], foreach, &var_name, prev_stmt)
    }

    fn check_find_foreach_body(
        &self,
        stmt: &Statement<'_>,
        foreach: &Foreach<'_>,
        var_name: &str,
        prev_stmt: &Statement<'_>,
    ) -> Option<Edit> {
        let Statement::If(if_stmt) = stmt else {
            return None;
        };

        let if_body_stmts = self.get_if_body_statements(&if_stmt.body)?;
        if if_body_stmts.len() != 2 {
            return None;
        }

        // First must be $var = $item (the foreach value variable);
        let Statement::Expression(assign_stmt) = &if_body_stmts[0] else {
            return None;
        };
        let Expression::Assignment(assign) = assign_stmt.expression else {
            return None;
        };
        if !matches!(assign.operator, AssignmentOperator::Assign(_)) {
            return None;
        }
        let assigned_var = self.get_simple_variable_name(&assign.lhs)?;
        if assigned_var != var_name {
            return None;
        }

        // The assigned value must be the foreach value variable
        let foreach_value_name = self.get_simple_variable_name(foreach.target.value())?;
        let assigned_value_name = self.get_simple_variable_name(&assign.rhs)?;
        if assigned_value_name != foreach_value_name {
            return None;
        }

        // Second must be break;
        let Statement::Break(break_stmt) = &if_body_stmts[1] else {
            return None;
        };
        if let Some(level) = &break_stmt.level {
            if !self.is_one_literal(level) {
                return None;
            }
        }

        let condition_source = self.get_text(if_stmt.condition.span());
        let value_var = self.get_foreach_value_var(foreach)?;
        let array_source = self.get_text(foreach.expression.span());

        let replacement = format!(
            "${} = array_find({}, fn({}) => {})",
            var_name, array_source, value_var, condition_source
        );

        let span = Span::new(
            prev_stmt.span().file_id,
            prev_stmt.span().start,
            foreach.span().end,
        );

        Some(Edit::new(
            span,
            replacement,
            "Convert foreach to array_find() (PHP 8.4+)",
        ))
    }

    fn get_if_body_statements<'a>(&self, body: &'a IfBody<'a>) -> Option<&'a [Statement<'a>]> {
        match body {
            IfBody::Statement(stmt_body) => {
                match stmt_body.statement {
                    Statement::Block(ref block) => Some(block.statements.as_slice()),
                    _ => None,
                }
            }
            IfBody::ColonDelimited(block) => Some(block.statements.as_slice()),
        }
    }

    fn get_foreach_value_var(&self, foreach: &Foreach<'_>) -> Option<String> {
        let value_expr = foreach.target.value();
        self.get_variable_text(value_expr)
    }

    fn get_variable_text(&self, expr: &Expression<'_>) -> Option<String> {
        if let Expression::Variable(Variable::Direct(var)) = expr {
            return Some(format!("${}", var.name.trim_start_matches('$')));
        }
        None
    }

    fn get_simple_variable_name(&self, expr: &Expression<'_>) -> Option<String> {
        if let Expression::Variable(Variable::Direct(var)) = expr {
            return Some(var.name.trim_start_matches('$').to_string());
        }
        None
    }

    fn is_null_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::Null(_)))
    }

    fn is_one_literal(&self, expr: &Expression<'_>) -> bool {
        if let Expression::Literal(Literal::Integer(int_lit)) = expr {
            return int_lit.value == Some(1);
        }
        false
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
                self.check_statement_sequence(block.statements.as_slice());
            }
        }
    }

    fn check_while_body(&mut self, body: &WhileBody<'_>) {
        match body {
            WhileBody::Statement(stmt) => self.check_statement(stmt),
            WhileBody::ColonDelimited(block) => self.check_statement_sequence(block.statements.as_slice()),
        }
    }

    fn check_for_body(&mut self, body: &ForBody<'_>) {
        match body {
            ForBody::Statement(stmt) => self.check_statement(stmt),
            ForBody::ColonDelimited(block) => self.check_statement_sequence(block.statements.as_slice()),
        }
    }

    fn check_foreach_body(&mut self, body: &ForeachBody<'_>) {
        match body {
            ForeachBody::Statement(stmt) => self.check_statement(stmt),
            ForeachBody::ColonDelimited(block) => self.check_statement_sequence(block.statements.as_slice()),
        }
    }
}

pub struct ForeachToArrayFindRule;

impl Rule for ForeachToArrayFindRule {
    fn name(&self) -> &'static str {
        "foreach_to_array_find"
    }

    fn description(&self) -> &'static str {
        "Convert foreach loops finding first match to array_find() (PHP 8.4+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_foreach_to_array_find(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php84)
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
        check_foreach_to_array_find(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_basic_pattern() {
        let source = r#"<?php
$found = null;
foreach ($animals as $animal) {
    if (str_starts_with($animal, 'c')) {
        $found = $animal;
        break;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$found = array_find($animals, fn($animal) => str_starts_with($animal, 'c'))"));
    }

    #[test]
    fn test_simple_condition() {
        let source = r#"<?php
$result = null;
foreach ($items as $item) {
    if ($item > 10) {
        $result = $item;
        break;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = array_find($items, fn($item) => $item > 10)"));
    }

    #[test]
    fn test_skip_wrong_initial_value() {
        // Initial value is not null
        let source = r#"<?php
$found = 0;
foreach ($animals as $animal) {
    if (str_starts_with($animal, 'c')) {
        $found = $animal;
        break;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_wrong_assignment() {
        // Assigns something other than the loop variable
        let source = r#"<?php
$found = null;
foreach ($animals as $animal) {
    if (str_starts_with($animal, 'c')) {
        $found = 'cat';
        break;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_break() {
        let source = r#"<?php
$found = null;
foreach ($animals as $animal) {
    if (str_starts_with($animal, 'c')) {
        $found = $animal;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_in_function() {
        let source = r#"<?php
function findFirst($items) {
    $found = null;
    foreach ($items as $item) {
        if ($item > 0) {
            $found = $item;
            break;
        }
    }
    return $found;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
