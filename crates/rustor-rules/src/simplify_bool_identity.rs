//! Rule: simplify_bool_identity (Simplification)
//!
//! Simplifies boolean identity comparisons.
//!
//! Example transformation:
//! ```php
//! // Before
//! if ($condition === true) { }
//! if ($condition === false) { }
//! if (true === $condition) { }
//!
//! // After
//! if ($condition) { }
//! if (!$condition) { }
//! if ($condition) { }
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_simplify_bool_identity<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyBoolIdentityVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyBoolIdentityVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SimplifyBoolIdentityVisitor<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn is_true_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::True(_)))
    }

    fn is_false_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::False(_)))
    }
}

impl<'a, 's> Visitor<'a> for SimplifyBoolIdentityVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(bin_op) = expr {
            // Only handle === comparisons (strict identity)
            if let BinaryOperator::Identical(_) = &bin_op.operator {
                // $expr === true -> $expr
                if self.is_true_literal(&bin_op.rhs) {
                    let left_text = self.get_text(bin_op.lhs.span());
                    self.edits.push(Edit::new(
                        expr.span(),
                        left_text.to_string(),
                        "Simplify `=== true` comparison".to_string(),
                    ));
                    return true;
                }

                // true === $expr -> $expr
                if self.is_true_literal(&bin_op.lhs) {
                    let right_text = self.get_text(bin_op.rhs.span());
                    self.edits.push(Edit::new(
                        expr.span(),
                        right_text.to_string(),
                        "Simplify `true ===` comparison".to_string(),
                    ));
                    return true;
                }

                // $expr === false -> !$expr
                if self.is_false_literal(&bin_op.rhs) {
                    let left_text = self.get_text(bin_op.lhs.span());
                    // Add parentheses if it's a complex expression
                    let replacement = if needs_parens(&bin_op.lhs) {
                        format!("!({})", left_text)
                    } else {
                        format!("!{}", left_text)
                    };
                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Simplify `=== false` comparison".to_string(),
                    ));
                    return true;
                }

                // false === $expr -> !$expr
                if self.is_false_literal(&bin_op.lhs) {
                    let right_text = self.get_text(bin_op.rhs.span());
                    let replacement = if needs_parens(&bin_op.rhs) {
                        format!("!({})", right_text)
                    } else {
                        format!("!{}", right_text)
                    };
                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Simplify `false ===` comparison".to_string(),
                    ));
                    return true;
                }
            }
        }
        true
    }
}

/// Check if an expression needs parentheses when negated
fn needs_parens(expr: &Expression<'_>) -> bool {
    !matches!(
        expr,
        Expression::Variable(_)
            | Expression::Literal(_)
            | Expression::Parenthesized(_)
            | Expression::Call(_)
            | Expression::Access(_)
            | Expression::ArrayAccess(_)
    )
}

pub struct SimplifyBoolIdentityRule;

impl SimplifyBoolIdentityRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimplifyBoolIdentityRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for SimplifyBoolIdentityRule {
    fn name(&self) -> &'static str {
        "simplify_bool_identity"
    }

    fn description(&self) -> &'static str {
        "Simplify boolean identity comparisons (=== true, === false)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_bool_identity(program, source)
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
        check_simplify_bool_identity(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_identical_true_right() {
        let source = r#"<?php
if ($condition === true) { echo "yes"; }
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("if ($condition)"));
    }

    #[test]
    fn test_identical_true_left() {
        let source = r#"<?php
if (true === $condition) { echo "yes"; }
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("if ($condition)"));
    }

    #[test]
    fn test_identical_false_right() {
        let source = r#"<?php
if ($condition === false) { echo "no"; }
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("if (!$condition)"));
    }

    #[test]
    fn test_identical_false_left() {
        let source = r#"<?php
if (false === $condition) { echo "no"; }
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("if (!$condition)"));
    }

    #[test]
    fn test_complex_expression_needs_parens() {
        let source = r#"<?php
$result = ($a && $b) === false;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("!($a && $b)"));
    }

    #[test]
    fn test_skip_non_identity() {
        let source = r#"<?php
if ($condition == true) { echo "yes"; }
"#;
        // == is not ===, we skip it (different semantics)
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_non_boolean_literal() {
        let source = r#"<?php
if ($x === 1) { echo "one"; }
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_simplifications() {
        let source = r#"<?php
$a = $x === true;
$b = false === $y;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }
}
