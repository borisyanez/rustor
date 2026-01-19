//! Rule: Replace strlen comparison with empty string comparison
//!
//! Comparing strlen to 0 can be simplified by comparing directly to empty string.
//!
//! Transformations:
//! - `strlen($x) === 0` → `$x === ''`
//! - `strlen($x) !== 0` → `$x !== ''`
//! - `strlen($x) > 0` → `$x !== ''`
//! - `strlen($x) < 1` → `$x === ''`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for strlen comparison to 0
pub fn check_strlen_to_empty_string<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = StrlenEmptyStringVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StrlenEmptyStringVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for StrlenEmptyStringVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            if let Some(edit) = try_simplify_strlen(binary, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Check if expression is a zero literal
fn is_zero(expr: &Expression<'_>, source: &str) -> bool {
    if let Expression::Literal(Literal::Integer(int_lit)) = expr {
        let span = int_lit.span();
        let text = &source[span.start.offset as usize..span.end.offset as usize];
        return text == "0";
    }
    false
}

/// Check if expression is a strlen function call and return the argument text
fn get_strlen_arg<'a>(expr: &'a Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        let func_name = if let Expression::Identifier(ident) = func_call.function {
            let span = ident.span();
            &source[span.start.offset as usize..span.end.offset as usize]
        } else {
            return None;
        };

        if !func_name.eq_ignore_ascii_case("strlen") {
            return None;
        }

        let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
        if args.len() != 1 {
            return None;
        }

        let arg_span = args[0].value().span();
        let arg_text = &source[arg_span.start.offset as usize..arg_span.end.offset as usize];

        return Some(arg_text.to_string());
    }
    None
}

/// Try to simplify strlen($x) === 0 to $x === ''
fn try_simplify_strlen(binary: &Binary<'_>, source: &str) -> Option<Edit> {
    let operator = &binary.operator;

    // Must be ===, !==, >, or <
    let is_identical = matches!(operator, BinaryOperator::Identical(_));
    let is_not_identical = matches!(operator, BinaryOperator::NotIdentical(_));
    let is_greater = matches!(operator, BinaryOperator::GreaterThan(_));
    let is_less = matches!(operator, BinaryOperator::LessThan(_));

    if !is_identical && !is_not_identical && !is_greater && !is_less {
        return None;
    }

    // Find which side is strlen and which is the number
    let (strlen_arg, strlen_on_left) = if let Some(arg) = get_strlen_arg(binary.lhs, source) {
        if is_zero(binary.rhs, source) {
            (arg, true)
        } else {
            return None;
        }
    } else if let Some(arg) = get_strlen_arg(binary.rhs, source) {
        if is_zero(binary.lhs, source) {
            (arg, false)
        } else {
            return None;
        }
    } else {
        return None;
    };

    // Determine the comparison based on operator and position
    // strlen($x) === 0  ->  $x === ''
    // strlen($x) !== 0  ->  $x !== ''
    // strlen($x) > 0    ->  $x !== ''
    // 0 < strlen($x)    ->  $x !== ''
    // strlen($x) < 1    ->  $x === ''  (but we're comparing to 0, so this would be strlen($x) < 0 which is always false)

    let comparison = if strlen_on_left {
        if is_identical {
            "==="
        } else if is_not_identical {
            "!=="
        } else if is_greater {
            // strlen > 0 means non-empty
            "!=="
        } else {
            // strlen < 0 is impossible, skip
            return None;
        }
    } else {
        // 0 on left: 0 === strlen, 0 !== strlen, 0 < strlen, 0 > strlen
        if is_identical {
            "==="
        } else if is_not_identical {
            "!=="
        } else if is_less {
            // 0 < strlen means non-empty
            "!=="
        } else {
            // 0 > strlen is impossible, skip
            return None;
        }
    };

    let binary_span = binary.span();
    let replacement = format!("{} {} ''", strlen_arg, comparison);

    Some(Edit::new(
        binary_span,
        replacement,
        "Simplify strlen comparison to empty string",
    ))
}

use crate::registry::{Category, Rule};

pub struct StrlenToEmptyStringRule;

impl Rule for StrlenToEmptyStringRule {
    fn name(&self) -> &'static str {
        "strlen_to_empty_string"
    }

    fn description(&self) -> &'static str {
        "Replace strlen comparison with empty string comparison"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_strlen_to_empty_string(program, source)
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
        check_strlen_to_empty_string(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_identical_zero() {
        let source = "<?php strlen($value) === 0;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $value === '';");
    }

    #[test]
    fn test_not_identical_zero() {
        let source = "<?php strlen($value) !== 0;";
        assert_eq!(transform(source), "<?php $value !== '';");
    }

    #[test]
    fn test_greater_than_zero() {
        let source = "<?php strlen($str) > 0;";
        assert_eq!(transform(source), "<?php $str !== '';");
    }

    #[test]
    fn test_zero_on_left() {
        let source = "<?php 0 === strlen($x);";
        assert_eq!(transform(source), "<?php $x === '';");
    }

    #[test]
    fn test_zero_less_than_strlen() {
        let source = "<?php 0 < strlen($text);";
        assert_eq!(transform(source), "<?php $text !== '';");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_condition() {
        let source = "<?php if (strlen($s) === 0) {}";
        assert_eq!(transform(source), "<?php if ($s === '') {}");
    }

    #[test]
    fn test_in_assignment() {
        let source = "<?php $empty = strlen($str) === 0;";
        assert_eq!(transform(source), "<?php $empty = $str === '';");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return strlen($val) !== 0;";
        assert_eq!(transform(source), "<?php return $val !== '';");
    }

    #[test]
    fn test_with_property() {
        let source = r#"<?php strlen($obj->name) === 0;"#;
        assert_eq!(transform(source), r#"<?php $obj->name === '';"#);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = strlen($x) === 0;
$b = strlen($y) !== 0;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_compare_to_other() {
        // Comparing to 1 or other non-zero values
        let source = "<?php strlen($x) === 1;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_equals() {
        // Loose equality - different semantics
        let source = "<?php strlen($x) == 0;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_mb_strlen() {
        let source = "<?php mb_strlen($x) === 0;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php count($x) === 0;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
