//! Rule: Remove unnecessary ternary expressions
//!
//! Simplify ternary expressions that return true/false based on a condition.
//!
//! Transformations:
//! - `$x ? true : false` → `(bool) $x`
//! - `$x ? false : true` → `!$x`
//! - `$a === $b ? true : false` → `$a === $b`
//! - `$a === $b ? false : true` → `$a !== $b`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for unnecessary ternary expressions
pub fn check_unnecessary_ternary<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = UnnecessaryTernaryVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct UnnecessaryTernaryVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for UnnecessaryTernaryVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Conditional(cond) = expr {
            if let Some(replacement) = try_transform_unnecessary_ternary(cond, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Simplify unnecessary ternary expression",
                ));
                return false;
            }
        }
        true
    }
}

/// Check if expression is a boolean literal (true or false)
fn is_bool_literal(expr: &Expression<'_>, source: &str) -> Option<bool> {
    if let Expression::Literal(Literal::True(_)) = expr {
        return Some(true);
    }
    if let Expression::Literal(Literal::False(_)) = expr {
        return Some(false);
    }
    // Also check for identifier true/false (case insensitive)
    if let Expression::Identifier(ident) = expr {
        let span = ident.span();
        let name = &source[span.start.offset as usize..span.end.offset as usize];
        if name.eq_ignore_ascii_case("true") {
            return Some(true);
        }
        if name.eq_ignore_ascii_case("false") {
            return Some(false);
        }
    }
    None
}

/// Check if condition is a comparison operator that can be inverted
fn get_inverted_comparison(expr: &Expression<'_>, source: &str) -> Option<String> {
    if let Expression::Binary(binary) = expr {
        let left_span = binary.lhs.span();
        let right_span = binary.rhs.span();
        let left = &source[left_span.start.offset as usize..left_span.end.offset as usize];
        let right = &source[right_span.start.offset as usize..right_span.end.offset as usize];

        let inverted_op = match &binary.operator {
            BinaryOperator::Identical(_) => "!==",
            BinaryOperator::NotIdentical(_) => "===",
            BinaryOperator::Equal(_) => "!=",
            BinaryOperator::NotEqual(_) => "==",
            BinaryOperator::LessThan(_) => ">=",
            BinaryOperator::LessThanOrEqual(_) => ">",
            BinaryOperator::GreaterThan(_) => "<=",
            BinaryOperator::GreaterThanOrEqual(_) => "<",
            _ => return None,
        };

        return Some(format!("{} {} {}", left, inverted_op, right));
    }
    None
}

/// Check if condition is already a boolean expression (comparison)
fn is_comparison(expr: &Expression<'_>) -> bool {
    if let Expression::Binary(binary) = expr {
        matches!(
            &binary.operator,
            BinaryOperator::Identical(_)
                | BinaryOperator::NotIdentical(_)
                | BinaryOperator::Equal(_)
                | BinaryOperator::NotEqual(_)
                | BinaryOperator::LessThan(_)
                | BinaryOperator::LessThanOrEqual(_)
                | BinaryOperator::GreaterThan(_)
                | BinaryOperator::GreaterThanOrEqual(_)
        )
    } else {
        false
    }
}

/// Try to transform unnecessary ternary, returning the replacement if successful
fn try_transform_unnecessary_ternary(cond: &Conditional<'_>, source: &str) -> Option<String> {
    // Must have explicit then branch (not short ternary)
    let then_expr = cond.then.as_ref()?;

    // Both branches must be boolean literals
    let then_bool = is_bool_literal(then_expr, source)?;
    let else_bool = is_bool_literal(cond.r#else, source)?;

    // They must be different (true/false or false/true)
    if then_bool == else_bool {
        return None;
    }

    let condition_span = cond.condition.span();
    let condition_text = &source[condition_span.start.offset as usize..condition_span.end.offset as usize];

    if then_bool {
        // $x ? true : false
        if is_comparison(cond.condition) {
            // Comparison already returns bool, just return it
            Some(condition_text.to_string())
        } else {
            // Need to cast to bool
            Some(format!("(bool) {}", condition_text))
        }
    } else {
        // $x ? false : true - need to negate
        if let Some(inverted) = get_inverted_comparison(cond.condition, source) {
            // Can invert the comparison operator
            Some(inverted)
        } else if is_comparison(cond.condition) {
            // Already a comparison, just negate it
            Some(format!("!({})", condition_text))
        } else {
            // General case - negate with !
            Some(format!("!{}", condition_text))
        }
    }
}

use crate::registry::{Category, Rule};

pub struct UnnecessaryTernaryRule;

impl Rule for UnnecessaryTernaryRule {
    fn name(&self) -> &'static str {
        "unnecessary_ternary"
    }

    fn description(&self) -> &'static str {
        "Simplify unnecessary ternary expressions like $x ? true : false"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_unnecessary_ternary(program, source)
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
        check_unnecessary_ternary(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== True/False Cases ====================

    #[test]
    fn test_var_true_false() {
        let source = "<?php $x ? true : false;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php (bool) $x;");
    }

    #[test]
    fn test_var_false_true() {
        let source = "<?php $x ? false : true;";
        assert_eq!(transform(source), "<?php !$x;");
    }

    #[test]
    fn test_comparison_true_false() {
        let source = "<?php $a === $b ? true : false;";
        assert_eq!(transform(source), "<?php $a === $b;");
    }

    #[test]
    fn test_comparison_false_true() {
        let source = "<?php $a === $b ? false : true;";
        assert_eq!(transform(source), "<?php $a !== $b;");
    }

    // ==================== Comparison Inversions ====================

    #[test]
    fn test_not_equal_inverted() {
        let source = "<?php $a != $b ? false : true;";
        assert_eq!(transform(source), "<?php $a == $b;");
    }

    #[test]
    fn test_less_than_inverted() {
        let source = "<?php $a < $b ? false : true;";
        assert_eq!(transform(source), "<?php $a >= $b;");
    }

    #[test]
    fn test_greater_than_inverted() {
        let source = "<?php $a > $b ? false : true;";
        assert_eq!(transform(source), "<?php $a <= $b;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = $x ? true : false;";
        assert_eq!(transform(source), "<?php $result = (bool) $x;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return $valid ? true : false;";
        assert_eq!(transform(source), "<?php return (bool) $valid;");
    }

    #[test]
    fn test_function_call_condition() {
        let source = "<?php isset($x) ? true : false;";
        // isset already returns bool
        assert_eq!(transform(source), "<?php (bool) isset($x);");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = $x ? true : false;
$b = $y ? false : true;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_not_bool_then() {
        let source = "<?php $x ? 'yes' : false;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_not_bool_else() {
        let source = "<?php $x ? true : 'no';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_same_value() {
        let source = "<?php $x ? true : true;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_short_ternary() {
        let source = "<?php $x ?: false;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_normal_ternary() {
        let source = "<?php $x ? 'yes' : 'no';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
