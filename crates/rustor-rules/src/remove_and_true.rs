//! Rule: remove_and_true (DeadCode)
//!
//! Removes `&& true` from boolean expressions as it has no effect.
//!
//! Example transformation:
//! ```php
//! // Before
//! if ($condition && true) { }
//! return true && $value;
//!
//! // After
//! if ($condition) { }
//! return $value;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_remove_and_true<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveAndTrueVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveAndTrueVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> RemoveAndTrueVisitor<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn is_true_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::True(_)))
    }
}

impl<'a, 's> Visitor<'a> for RemoveAndTrueVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(bin_op) = expr {
            if let BinaryOperator::And(_) = &bin_op.operator {
                // Check if left is true: `true && $expr` -> `$expr`
                if self.is_true_literal(&bin_op.lhs) {
                    let right_text = self.get_text(bin_op.rhs.span());
                    self.edits.push(Edit::new(
                        expr.span(),
                        right_text.to_string(),
                        "Remove redundant `true &&`".to_string(),
                    ));
                    return true;
                }

                // Check if right is true: `$expr && true` -> `$expr`
                if self.is_true_literal(&bin_op.rhs) {
                    let left_text = self.get_text(bin_op.lhs.span());
                    self.edits.push(Edit::new(
                        expr.span(),
                        left_text.to_string(),
                        "Remove redundant `&& true`".to_string(),
                    ));
                    return true;
                }
            }
        }
        true
    }
}

pub struct RemoveAndTrueRule;

impl RemoveAndTrueRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RemoveAndTrueRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RemoveAndTrueRule {
    fn name(&self) -> &'static str {
        "remove_and_true"
    }

    fn description(&self) -> &'static str {
        "Remove redundant && true from boolean expressions"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_and_true(program, source)
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
        check_remove_and_true(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_and_true_right() {
        let source = r#"<?php
$result = $condition && true;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = $condition;"));
    }

    #[test]
    fn test_true_and_left() {
        let source = r#"<?php
$result = true && $condition;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = $condition;"));
    }

    #[test]
    fn test_in_if_condition() {
        let source = r#"<?php
if ($x > 0 && true) {
    echo "yes";
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("if ($x > 0)"));
    }

    #[test]
    fn test_skip_and_false() {
        let source = r#"<?php
$result = $condition && false;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_and_variable() {
        let source = r#"<?php
$result = $a && $b;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_and_true() {
        let source = r#"<?php
$a = $x && true;
$b = true && $y;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_complex_expression() {
        let source = r#"<?php
$result = ($a > $b) && true;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = ($a > $b);"));
    }
}
