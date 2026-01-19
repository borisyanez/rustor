//! Rule: Simplify tautology ternary to value
//!
//! When a ternary compares two values and returns one of them based on
//! whether they're equal or not, simplify to just return the appropriate value.
//!
//! Transformation:
//! - `($a !== $b) ? $a : $b` → `$a` (if not equal, return $a; else return $b which equals $a)
//! - `($a === $b) ? $a : $b` → `$b` (if equal, return $a which equals $b; else return $b)

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for tautology ternary patterns
pub fn check_simplify_tautology_ternary<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyTautologyTernaryVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyTautologyTernaryVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SimplifyTautologyTernaryVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Conditional(cond) = expr {
            if let Some(replacement) = try_simplify_tautology(cond, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Simplify tautology ternary to value",
                ));
                return false;
            }
        }
        true
    }
}

/// Extract text from an expression
fn expr_text(expr: &Expression<'_>, source: &str) -> String {
    let span = expr.span();
    source[span.start.offset as usize..span.end.offset as usize].to_string()
}

/// Check if two expressions are textually identical
fn exprs_equal(a: &Expression<'_>, b: &Expression<'_>, source: &str) -> bool {
    expr_text(a, source) == expr_text(b, source)
}

/// Unwrap parenthesized expressions to get the inner expression
fn unwrap_parens<'a>(expr: &'a Expression<'a>) -> &'a Expression<'a> {
    match expr {
        Expression::Parenthesized(paren) => unwrap_parens(paren.expression),
        _ => expr,
    }
}

/// Try to simplify a tautology ternary, returning the replacement if successful
fn try_simplify_tautology(cond: &Conditional<'_>, source: &str) -> Option<String> {
    // Must have explicit then branch
    let then_expr = cond.then.as_ref()?;

    // Condition must be === or !== (unwrap any parentheses first)
    let condition_inner = unwrap_parens(cond.condition);
    let binary = if let Expression::Binary(binary) = condition_inner {
        binary
    } else {
        return None;
    };

    let is_identical = matches!(binary.operator, BinaryOperator::Identical(_));
    let is_not_identical = matches!(binary.operator, BinaryOperator::NotIdentical(_));

    if !is_identical && !is_not_identical {
        return None;
    }

    // Check for pattern: ($a !== $b) ? $a : $b  or  ($a === $b) ? $a : $b
    // where $a matches one side of condition and $b matches the other

    // Case 1: condition.lhs matches then, condition.rhs matches else
    if exprs_equal(binary.lhs, then_expr, source) && exprs_equal(binary.rhs, cond.r#else, source) {
        // ($a !== $b) ? $a : $b => $a  (regardless of equality, we always get the "then" branch value)
        // ($a === $b) ? $a : $b => $b  (if equal, $a = $b; if not equal, $b)
        let result = if is_not_identical {
            expr_text(then_expr, source)
        } else {
            expr_text(cond.r#else, source)
        };
        return Some(result);
    }

    // Case 2: condition.rhs matches then, condition.lhs matches else
    if exprs_equal(binary.rhs, then_expr, source) && exprs_equal(binary.lhs, cond.r#else, source) {
        // ($a !== $b) ? $b : $a => $b
        // ($a === $b) ? $b : $a => $a
        let result = if is_not_identical {
            expr_text(then_expr, source)
        } else {
            expr_text(cond.r#else, source)
        };
        return Some(result);
    }

    None
}

use crate::registry::{Category, Rule};

pub struct SimplifyTautologyTernaryRule;

impl Rule for SimplifyTautologyTernaryRule {
    fn name(&self) -> &'static str {
        "simplify_tautology_ternary"
    }

    fn description(&self) -> &'static str {
        "Simplify tautology ternary to value"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_tautology_ternary(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
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
        check_simplify_tautology_ternary(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Not Identical Patterns ====================

    #[test]
    fn test_not_identical_basic() {
        // ($a !== $b) ? $a : $b => $a (always get the value that would be in "then")
        let source = "<?php ($a !== $b) ? $a : $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $a;");
    }

    #[test]
    fn test_not_identical_swapped() {
        // ($a !== $b) ? $b : $a => $b
        let source = "<?php ($a !== $b) ? $b : $a;";
        assert_eq!(transform(source), "<?php $b;");
    }

    // ==================== Identical Patterns ====================

    #[test]
    fn test_identical_basic() {
        // ($a === $b) ? $a : $b => $b
        let source = "<?php ($a === $b) ? $a : $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $b;");
    }

    #[test]
    fn test_identical_swapped() {
        // ($a === $b) ? $b : $a => $a
        let source = "<?php ($a === $b) ? $b : $a;";
        assert_eq!(transform(source), "<?php $a;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = ($x !== $y) ? $x : $y;";
        assert_eq!(transform(source), "<?php $result = $x;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return ($fullyQualified !== $short) ? $fullyQualified : $short;";
        assert_eq!(transform(source), "<?php return $fullyQualified;");
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn test_with_properties() {
        let source = "<?php ($obj->a !== $obj->b) ? $obj->a : $obj->b;";
        assert_eq!(transform(source), "<?php $obj->a;");
    }

    #[test]
    fn test_with_array_access() {
        let source = "<?php ($arr[0] !== $arr[1]) ? $arr[0] : $arr[1];";
        assert_eq!(transform(source), "<?php $arr[0];");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = ($x !== $y) ? $x : $y;
$b = ($p === $q) ? $p : $q;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_different_values() {
        // Values don't match the condition sides
        let source = "<?php ($a !== $b) ? $c : $d;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_comparison() {
        let source = "<?php $flag ? $a : $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_equality() {
        // Only === and !== are handled, not == and !=
        let source = "<?php ($a == $b) ? $a : $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_partial_match() {
        // Only one side matches
        let source = "<?php ($a !== $b) ? $a : $c;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
