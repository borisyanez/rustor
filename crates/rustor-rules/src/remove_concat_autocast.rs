//! Rule: Remove unnecessary (string) cast in concatenation
//!
//! String concatenation automatically converts values to strings,
//! so explicit (string) casts are redundant.
//!
//! Transformation:
//! - `'hi ' . (string) $value` → `'hi ' . $value`
//! - `(string) $a . $b` → `$a . $b`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for redundant string casts in concatenation
pub fn check_remove_concat_autocast<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveConcatAutocastVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveConcatAutocastVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RemoveConcatAutocastVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            // Check for concat operator
            if matches!(binary.operator, BinaryOperator::StringConcat(_)) {
                // Check left side for (string) cast
                if let Some(edit) = try_remove_string_cast(binary.lhs, self.source) {
                    self.edits.push(edit);
                }
                // Check right side for (string) cast
                if let Some(edit) = try_remove_string_cast(binary.rhs, self.source) {
                    self.edits.push(edit);
                }
            }
        }
        true
    }
}

/// Check if expression is a (string) cast and return edit to remove it
fn try_remove_string_cast(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    // String casts are UnaryPrefix expressions with StringCast operator
    if let Expression::UnaryPrefix(unary) = expr {
        if let UnaryPrefixOperator::StringCast(_, _) = &unary.operator {
            // Get the inner expression text
            let inner_span = unary.operand.span();
            let inner_text = &source[inner_span.start.offset as usize..inner_span.end.offset as usize];

            // Check if we need parentheses (for binary ops inside)
            let replacement = if needs_parentheses(unary.operand) {
                format!("({})", inner_text)
            } else {
                inner_text.to_string()
            };

            return Some(Edit::new(
                expr.span(),
                replacement,
                "Remove unnecessary (string) cast in concatenation",
            ));
        }
    }
    None
}

/// Check if the expression needs parentheses when removing the cast
fn needs_parentheses(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::Binary(_)
            | Expression::Conditional(_)
            | Expression::Assignment(_)
    )
}

use crate::registry::{Category, Rule};

pub struct RemoveConcatAutocastRule;

impl Rule for RemoveConcatAutocastRule {
    fn name(&self) -> &'static str {
        "remove_concat_autocast"
    }

    fn description(&self) -> &'static str {
        "Remove unnecessary (string) cast in concatenation"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_concat_autocast(program, source)
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
        check_remove_concat_autocast(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_cast_on_right() {
        let source = "<?php 'hi ' . (string) $value;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php 'hi ' . $value;");
    }

    #[test]
    fn test_cast_on_left() {
        let source = "<?php (string) $value . ' world';";
        assert_eq!(transform(source), "<?php $value . ' world';");
    }

    #[test]
    fn test_cast_on_both() {
        let source = "<?php (string) $a . (string) $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a . $b;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = 'Value: ' . (string) $num;";
        assert_eq!(transform(source), "<?php $result = 'Value: ' . $num;");
    }

    #[test]
    fn test_in_echo() {
        let source = "<?php echo 'Count: ' . (string) $count;";
        assert_eq!(transform(source), "<?php echo 'Count: ' . $count;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return 'ID: ' . (string) $id;";
        assert_eq!(transform(source), "<?php return 'ID: ' . $id;");
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn test_with_function_call() {
        let source = "<?php 'Result: ' . (string) getValue();";
        assert_eq!(transform(source), "<?php 'Result: ' . getValue();");
    }

    #[test]
    fn test_with_array_access() {
        let source = "<?php 'Item: ' . (string) $arr[0];";
        assert_eq!(transform(source), "<?php 'Item: ' . $arr[0];");
    }

    #[test]
    fn test_with_property() {
        let source = "<?php 'Name: ' . (string) $obj->name;";
        assert_eq!(transform(source), "<?php 'Name: ' . $obj->name;");
    }

    // ==================== Preserve Parentheses ====================

    #[test]
    fn test_binary_op_needs_parens() {
        let source = "<?php 'Sum: ' . (string) ($a + $b);";
        assert_eq!(transform(source), "<?php 'Sum: ' . ($a + $b);");
    }

    // ==================== Multiple Concats ====================

    #[test]
    fn test_chain() {
        let source = "<?php 'a' . (string) $x . 'b' . (string) $y;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_int_cast() {
        let source = "<?php 'Num: ' . (int) $value;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_bool_cast() {
        let source = "<?php 'Bool: ' . (bool) $value;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_concat() {
        let source = "<?php (string) $value;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_addition() {
        // String cast in addition context is different
        let source = "<?php 1 + (string) $value;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
