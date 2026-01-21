//! Rule: remove_or_true (Simplification)
//!
//! Simplifies `$expr || true` to `true` since OR with true is always true.
//!
//! Example transformation:
//! ```php
//! // Before
//! $result = $condition || true;
//! if (true || $condition) { }
//!
//! // After
//! $result = true;
//! if (true) { }
//! ```
//!
//! Note: This removes a potential side effect from the left-hand expression
//! when it's on the left side. Use with caution.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_remove_or_true<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveOrTrueVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveOrTrueVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> RemoveOrTrueVisitor<'s> {
    #[allow(dead_code)]
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn is_true_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::True(_)))
    }
}

impl<'a, 's> Visitor<'a> for RemoveOrTrueVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(bin_op) = expr {
            if let BinaryOperator::Or(_) = &bin_op.operator {
                // $expr || true -> true
                if self.is_true_literal(&bin_op.rhs) {
                    self.edits.push(Edit::new(
                        expr.span(),
                        "true".to_string(),
                        "Simplify `|| true` to `true`".to_string(),
                    ));
                    return true;
                }

                // true || $expr -> true
                if self.is_true_literal(&bin_op.lhs) {
                    self.edits.push(Edit::new(
                        expr.span(),
                        "true".to_string(),
                        "Simplify `true ||` to `true`".to_string(),
                    ));
                    return true;
                }
            }
        }
        true
    }
}

pub struct RemoveOrTrueRule;

impl RemoveOrTrueRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RemoveOrTrueRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RemoveOrTrueRule {
    fn name(&self) -> &'static str {
        "remove_or_true"
    }

    fn description(&self) -> &'static str {
        "Simplify || true to just true"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_or_true(program, source)
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
        check_remove_or_true(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_or_true_right() {
        let source = r#"<?php
$result = $condition || true;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = true;"));
    }

    #[test]
    fn test_true_or_left() {
        let source = r#"<?php
$result = true || $condition;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = true;"));
    }

    #[test]
    fn test_in_if_condition() {
        let source = r#"<?php
if ($x > 0 || true) {
    echo "always";
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("if (true)"));
    }

    #[test]
    fn test_skip_or_false() {
        let source = r#"<?php
$result = $condition || false;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_or_variable() {
        let source = r#"<?php
$result = $a || $b;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_or_true() {
        let source = r#"<?php
$a = $x || true;
$b = true || $y;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }
}
