//! Rule: simplify_useless_concat (Simplification)
//!
//! Removes useless string concatenation with empty strings.
//!
//! Example transformation:
//! ```php
//! // Before
//! $result = $str . '';
//! $result = '' . $str;
//!
//! // After
//! $result = $str;
//! $result = $str;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_simplify_useless_concat<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyUselessConcatVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyUselessConcatVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SimplifyUselessConcatVisitor<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn is_empty_string(&self, expr: &Expression<'_>) -> bool {
        if let Expression::Literal(Literal::String(string_lit)) = expr {
            let text = self.get_text(string_lit.span());
            // Check for '' or ""
            text == "''" || text == "\"\""
        } else {
            false
        }
    }
}

impl<'a, 's> Visitor<'a> for SimplifyUselessConcatVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(bin_op) = expr {
            if let BinaryOperator::StringConcat(_) = &bin_op.operator {
                // Check if left is empty string: `'' . $expr` -> `$expr`
                if self.is_empty_string(&bin_op.lhs) {
                    let right_text = self.get_text(bin_op.rhs.span());
                    self.edits.push(Edit::new(
                        expr.span(),
                        right_text.to_string(),
                        "Remove redundant `'' .`".to_string(),
                    ));
                    return true;
                }

                // Check if right is empty string: `$expr . ''` -> `$expr`
                if self.is_empty_string(&bin_op.rhs) {
                    let left_text = self.get_text(bin_op.lhs.span());
                    self.edits.push(Edit::new(
                        expr.span(),
                        left_text.to_string(),
                        "Remove redundant `. ''`".to_string(),
                    ));
                    return true;
                }
            }
        }
        true
    }
}

pub struct SimplifyUselessConcatRule;

impl SimplifyUselessConcatRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimplifyUselessConcatRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for SimplifyUselessConcatRule {
    fn name(&self) -> &'static str {
        "simplify_useless_concat"
    }

    fn description(&self) -> &'static str {
        "Remove useless string concatenation with empty strings"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_useless_concat(program, source)
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
        check_simplify_useless_concat(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_concat_empty_right_single_quote() {
        let source = r#"<?php
$result = $str . '';
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = $str;"));
    }

    #[test]
    fn test_concat_empty_right_double_quote() {
        let source = r#"<?php
$result = $str . "";
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = $str;"));
    }

    #[test]
    fn test_concat_empty_left() {
        let source = r#"<?php
$result = '' . $str;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$result = $str;"));
    }

    #[test]
    fn test_skip_non_empty_string() {
        let source = r#"<?php
$result = $str . 'suffix';
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_variable_concat() {
        let source = r#"<?php
$result = $a . $b;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_useless_concat() {
        let source = r#"<?php
$a = $x . '';
$b = '' . $y;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }
}
