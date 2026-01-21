//! Rule: remove_or_false (Simplification)
//!
//! Removes `|| false` from boolean expressions as it has no effect.
//!
//! Example transformation:
//! ```php
//! // Before
//! if ($condition || false) { }
//! return $value || false;
//!
//! // After
//! if ($condition) { }
//! return $value;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_remove_or_false<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveOrFalseVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveOrFalseVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> RemoveOrFalseVisitor<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn is_false_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::False(_)))
    }
}

impl<'a, 's> Visitor<'a> for RemoveOrFalseVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(bin_op) = expr {
            if let BinaryOperator::Or(_) = &bin_op.operator {
                // Check if left is false: `false || $expr` -> `$expr`
                if self.is_false_literal(&bin_op.lhs) {
                    let right_text = self.get_text(bin_op.rhs.span());
                    self.edits.push(Edit::new(
                        expr.span(),
                        right_text.to_string(),
                        "Remove redundant `false ||`".to_string(),
                    ));
                    return true;
                }

                // Check if right is false: `$expr || false` -> `$expr`
                if self.is_false_literal(&bin_op.rhs) {
                    let left_text = self.get_text(bin_op.lhs.span());
                    self.edits.push(Edit::new(
                        expr.span(),
                        left_text.to_string(),
                        "Remove redundant `|| false`".to_string(),
                    ));
                    return true;
                }
            }
        }
        true
    }
}

pub struct RemoveOrFalseRule;

impl RemoveOrFalseRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RemoveOrFalseRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RemoveOrFalseRule {
    fn name(&self) -> &'static str {
        "remove_or_false"
    }

    fn description(&self) -> &'static str {
        "Remove redundant || false from boolean expressions"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_or_false(program, source)
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
        check_remove_or_false(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_or_false_right() {
        let source = r#"<?php
$result = $condition || false;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = $condition;"));
    }

    #[test]
    fn test_false_or_left() {
        let source = r#"<?php
$result = false || $condition;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = $condition;"));
    }

    #[test]
    fn test_in_if_condition() {
        let source = r#"<?php
if ($x > 0 || false) {
    echo "yes";
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("if ($x > 0)"));
    }

    #[test]
    fn test_skip_or_true() {
        let source = r#"<?php
$result = $condition || true;
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
    fn test_multiple_or_false() {
        let source = r#"<?php
$a = $x || false;
$b = false || $y;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }
}
