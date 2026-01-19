//! Rule: Simplify array_search comparison to in_array
//!
//! When checking if a value exists in an array, in_array is more readable than
//! comparing array_search result to false.
//!
//! Transformations:
//! - `array_search($x, $arr) !== false` → `in_array($x, $arr)`
//! - `array_search($x, $arr) === false` → `!in_array($x, $arr)`
//! - `array_search($x, $arr, true) !== false` → `in_array($x, $arr, true)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for array_search comparisons to false
pub fn check_simplify_array_search<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyArraySearchVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyArraySearchVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SimplifyArraySearchVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            if let Some(edit) = try_simplify_array_search(binary, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Check if expression is a false literal
fn is_false_literal(expr: &Expression<'_>, source: &str) -> bool {
    if let Expression::Literal(Literal::False(_)) = expr {
        return true;
    }
    // Also check for lowercase 'false' constant
    if let Expression::Identifier(ident) = expr {
        let span = ident.span();
        let text = &source[span.start.offset as usize..span.end.offset as usize];
        return text.eq_ignore_ascii_case("false");
    }
    false
}

/// Check if expression is an array_search function call and return its args text
fn get_array_search_args<'a>(expr: &'a Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        let func_name = if let Expression::Identifier(ident) = func_call.function {
            let span = ident.span();
            &source[span.start.offset as usize..span.end.offset as usize]
        } else {
            return None;
        };

        if !func_name.eq_ignore_ascii_case("array_search") {
            return None;
        }

        // Get the arguments list
        let arg_list_span = func_call.argument_list.span();
        let args_text = &source[arg_list_span.start.offset as usize..arg_list_span.end.offset as usize];

        return Some(args_text.to_string());
    }
    None
}

/// Try to simplify array_search !== false or === false
fn try_simplify_array_search(binary: &Binary<'_>, source: &str) -> Option<Edit> {
    // Must be === or !==
    let is_identical = matches!(binary.operator, BinaryOperator::Identical(_));
    let is_not_identical = matches!(binary.operator, BinaryOperator::NotIdentical(_));

    if !is_identical && !is_not_identical {
        return None;
    }

    // One side should be array_search, other should be false
    let (args_text, _) = if let Some(args) = get_array_search_args(binary.lhs, source) {
        if is_false_literal(binary.rhs, source) {
            (args, binary.lhs)
        } else {
            return None;
        }
    } else if let Some(args) = get_array_search_args(binary.rhs, source) {
        if is_false_literal(binary.lhs, source) {
            (args, binary.rhs)
        } else {
            return None;
        }
    } else {
        return None;
    };

    let binary_span = binary.span();

    // Create in_array call with same arguments
    let replacement = if is_not_identical {
        // !== false means "exists" -> in_array(...)
        format!("in_array{}", args_text)
    } else {
        // === false means "not exists" -> !in_array(...)
        format!("!in_array{}", args_text)
    };

    Some(Edit::new(
        binary_span,
        replacement,
        "Simplify array_search to in_array",
    ))
}

use crate::registry::{Category, Rule};

pub struct SimplifyArraySearchRule;

impl Rule for SimplifyArraySearchRule {
    fn name(&self) -> &'static str {
        "simplify_array_search"
    }

    fn description(&self) -> &'static str {
        "Simplify array_search comparison to in_array"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_array_search(program, source)
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
        check_simplify_array_search(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_not_identical_false() {
        let source = r#"<?php array_search("searching", $array) !== false;"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), r#"<?php in_array("searching", $array);"#);
    }

    #[test]
    fn test_identical_false() {
        let source = r#"<?php array_search("searching", $array) === false;"#;
        assert_eq!(transform(source), r#"<?php !in_array("searching", $array);"#);
    }

    #[test]
    fn test_with_strict_flag() {
        let source = r#"<?php array_search("searching", $array, true) !== false;"#;
        assert_eq!(transform(source), r#"<?php in_array("searching", $array, true);"#);
    }

    #[test]
    fn test_false_on_left() {
        let source = r#"<?php false !== array_search($val, $arr);"#;
        assert_eq!(transform(source), r#"<?php in_array($val, $arr);"#);
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_condition() {
        let source = r#"<?php if (array_search($x, $arr) !== false) {}"#;
        assert_eq!(transform(source), r#"<?php if (in_array($x, $arr)) {}"#);
    }

    #[test]
    fn test_in_assignment() {
        let source = r#"<?php $exists = array_search($needle, $haystack) !== false;"#;
        assert_eq!(transform(source), r#"<?php $exists = in_array($needle, $haystack);"#);
    }

    #[test]
    fn test_in_return() {
        let source = r#"<?php return array_search($val, $items) !== false;"#;
        assert_eq!(transform(source), r#"<?php return in_array($val, $items);"#);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = array_search($x, $arr1) !== false;
$b = array_search($y, $arr2) === false;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_compare_to_other() {
        // Comparing to something other than false
        let source = "<?php array_search($x, $arr) !== null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_equals() {
        // == instead of ===
        let source = "<?php array_search($x, $arr) == false;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_not_equals() {
        // != instead of !==
        let source = "<?php array_search($x, $arr) != false;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php in_array($x, $arr) !== false;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_compare_to_true() {
        let source = "<?php array_search($x, $arr) === true;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
