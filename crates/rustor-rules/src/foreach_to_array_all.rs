//! Rule: Convert foreach loops to array_all() (PHP 8.4+)
//!
//! Converts foreach loops that check if all elements match a condition to array_all().
//!
//! Pattern 1 (Boolean assignment with break):
//! ```php
//! // Before
//! $allMatch = true;
//! foreach ($animals as $animal) {
//!     if (!str_starts_with($animal, 'c')) {
//!         $allMatch = false;
//!         break;
//!     }
//! }
//!
//! // After
//! $allMatch = array_all($animals, fn($animal) => str_starts_with($animal, 'c'));
//! ```
//!
//! Pattern 2 (Early return):
//! ```php
//! // Before
//! foreach ($animals as $animal) {
//!     if (!str_starts_with($animal, 'c')) {
//!         return false;
//!     }
//! }
//! return true;
//!
//! // After
//! return array_all($animals, fn($animal) => str_starts_with($animal, 'c'));
//! ```

use mago_span::{HasSpan, Span};
use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for foreach to array_all conversions
pub fn check_foreach_to_array_all<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut checker = ForeachToArrayAllChecker {
        source,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct ForeachToArrayAllChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> ForeachToArrayAllChecker<'s> {
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
            // Pattern 1: $found = true; foreach (...) { if (!...) { $found = false; break; } }
            if i > 0 {
                if let Some(edit) = self.check_boolean_assignment_pattern(&statements[i - 1], &statements[i]) {
                    self.edits.push(edit);
                    continue;
                }
            }

            // Pattern 2: foreach (...) { if (!...) { return false; } } return true;
            if i + 1 < statements.len() {
                if let Some(edit) = self.check_early_return_pattern(&statements[i], &statements[i + 1]) {
                    self.edits.push(edit);
                }
            }
        }
    }

    fn check_boolean_assignment_pattern(
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
        // Must assign true (opposite of array_any)
        if !self.is_true_literal(&prev_assign.rhs) {
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
            return self.check_boolean_assignment_foreach_body(first_stmt, foreach, &var_name, prev_stmt);
        };

        let block_stmts = block.statements.as_slice();
        if block_stmts.len() != 1 {
            return None;
        }

        self.check_boolean_assignment_foreach_body(&block_stmts[0], foreach, &var_name, prev_stmt)
    }

    fn check_boolean_assignment_foreach_body(
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

        // First must be $var = false;
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
        // Must assign false (opposite of array_any)
        if !self.is_false_literal(&assign.rhs) {
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

        // The condition should be negated - we'll negate it back
        let condition_source = self.negate_condition_source(&if_stmt.condition);
        let value_var = self.get_foreach_value_var(foreach)?;
        let array_source = self.get_text(foreach.expression.span());

        let replacement = format!(
            "${} = array_all({}, fn({}) => {})",
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
            "Convert foreach to array_all() (PHP 8.4+)",
        ))
    }

    fn check_early_return_pattern(
        &self,
        foreach_stmt: &Statement<'_>,
        next_stmt: &Statement<'_>,
    ) -> Option<Edit> {
        let Statement::Foreach(foreach) = foreach_stmt else {
            return None;
        };

        // Next statement must be return true; (opposite of array_any)
        let Statement::Return(return_stmt) = next_stmt else {
            return None;
        };
        let return_value = return_stmt.value.as_ref()?;
        if !self.is_true_literal(return_value) {
            return None;
        }

        let body_stmts = foreach.body.statements();
        if body_stmts.len() != 1 {
            return None;
        }

        let if_stmt = self.get_if_from_statement(&body_stmts[0])?;

        let if_body_stmts = self.get_if_body_statements(&if_stmt.body)?;
        if if_body_stmts.len() != 1 {
            return None;
        }

        let Statement::Return(inner_return) = &if_body_stmts[0] else {
            return None;
        };
        let inner_return_value = inner_return.value.as_ref()?;
        // Must return false (opposite of array_any)
        if !self.is_false_literal(inner_return_value) {
            return None;
        }

        // The condition should be negated - we'll negate it back
        let condition_source = self.negate_condition_source(&if_stmt.condition);
        let value_var = self.get_foreach_value_var(foreach)?;
        let array_source = self.get_text(foreach.expression.span());

        let replacement = format!(
            "return array_all({}, fn({}) => {})",
            array_source, value_var, condition_source
        );

        let span = Span::new(
            foreach.span().file_id,
            foreach.span().start,
            next_stmt.span().end,
        );

        Some(Edit::new(
            span,
            replacement,
            "Convert foreach to array_all() (PHP 8.4+)",
        ))
    }

    /// Negate a condition - if it starts with !, remove it; otherwise add !
    fn negate_condition_source(&self, condition: &Expression<'_>) -> String {
        if let Expression::UnaryPrefix(unary) = condition {
            if let UnaryPrefixOperator::Not(_) = unary.operator {
                // Already negated, just return the inner expression
                return self.get_text(unary.operand.span()).to_string();
            }
        }
        // Not negated, add !
        format!("!({})", self.get_text(condition.span()))
    }

    fn get_if_from_statement<'a>(&self, stmt: &'a Statement<'a>) -> Option<&'a If<'a>> {
        match stmt {
            Statement::If(if_stmt) => Some(if_stmt),
            Statement::Block(block) => {
                let stmts = block.statements.as_slice();
                if stmts.len() == 1 {
                    if let Statement::If(if_stmt) = &stmts[0] {
                        return Some(if_stmt);
                    }
                }
                None
            }
            _ => None,
        }
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

    fn is_false_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::False(_)))
    }

    fn is_true_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::True(_)))
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

pub struct ForeachToArrayAllRule;

impl Rule for ForeachToArrayAllRule {
    fn name(&self) -> &'static str {
        "foreach_to_array_all"
    }

    fn description(&self) -> &'static str {
        "Convert foreach loops checking all match to array_all() (PHP 8.4+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_foreach_to_array_all(program, source)
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
        check_foreach_to_array_all(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Early Return Pattern Tests ====================

    #[test]
    fn test_early_return_pattern() {
        let source = r#"<?php
foreach ($animals as $animal) {
    if (!str_starts_with($animal, 'c')) {
        return false;
    }
}
return true;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("return array_all($animals, fn($animal) => str_starts_with($animal, 'c'))"));
    }

    #[test]
    fn test_early_return_simple_condition() {
        let source = r#"<?php
foreach ($items as $item) {
    if (!($item > 10)) {
        return false;
    }
}
return true;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("return array_all($items, fn($item) => ($item > 10))"));
    }

    // ==================== Boolean Assignment Pattern Tests ====================

    #[test]
    fn test_boolean_assignment_pattern() {
        let source = r#"<?php
$allMatch = true;
foreach ($animals as $animal) {
    if (!str_starts_with($animal, 'c')) {
        $allMatch = false;
        break;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$allMatch = array_all($animals, fn($animal) => str_starts_with($animal, 'c'))"));
    }

    #[test]
    fn test_boolean_assignment_non_negated_condition() {
        // When condition is not negated, we add ! around it
        let source = r#"<?php
$allSmall = true;
foreach ($values as $value) {
    if ($value > 100) {
        $allSmall = false;
        break;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$allSmall = array_all($values, fn($value) => !($value > 100))"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_wrong_initial_value() {
        // Initial value is false, not true
        let source = r#"<?php
$allMatch = false;
foreach ($animals as $animal) {
    if (!str_starts_with($animal, 'c')) {
        $allMatch = false;
        break;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_wrong_return_value() {
        // Returns false instead of true after loop
        let source = r#"<?php
foreach ($animals as $animal) {
    if (!str_starts_with($animal, 'c')) {
        return false;
    }
}
return false;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== In Context Tests ====================

    #[test]
    fn test_in_function() {
        let source = r#"<?php
function allStartWith($animals, $prefix) {
    foreach ($animals as $animal) {
        if (!str_starts_with($animal, $prefix)) {
            return false;
        }
    }
    return true;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
