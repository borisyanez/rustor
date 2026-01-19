//! Rule: Simplify redundant ternary with implode
//!
//! When checking if an array is empty and returning empty string vs implode,
//! the ternary is redundant since implode([]) returns ''.
//!
//! Transformation:
//! - `$arr === [] ? '' : implode(',', $arr)` → `implode(',', $arr)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for redundant ternary implode patterns
pub fn check_ternary_implode_to_implode<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = TernaryImplodeToImplodeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct TernaryImplodeToImplodeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for TernaryImplodeToImplodeVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Conditional(cond) = expr {
            if let Some(replacement) = try_simplify_ternary_implode(cond, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Simplify redundant ternary with implode",
                ));
                return false;
            }
        }
        true
    }
}

/// Get variable name or expression text
fn get_expr_text(expr: &Expression<'_>, source: &str) -> String {
    let span = expr.span();
    source[span.start.offset as usize..span.end.offset as usize].to_string()
}

/// Check if expression is an empty array literal []
fn is_empty_array(expr: &Expression<'_>) -> bool {
    if let Expression::Array(array) = expr {
        return array.elements.is_empty();
    }
    false
}

/// Check if expression is an empty string literal ''
fn is_empty_string(expr: &Expression<'_>, source: &str) -> bool {
    if let Expression::Literal(Literal::String(string_lit)) = expr {
        let span = string_lit.span();
        let raw = &source[span.start.offset as usize..span.end.offset as usize];
        // Check for '' or ""
        return raw == "''" || raw == "\"\"";
    }
    false
}

/// Check if this is an implode/join function call and get the second argument
fn get_implode_second_arg<'a>(expr: &'a Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        let func_name = if let Expression::Identifier(ident) = func_call.function {
            let span = ident.span();
            &source[span.start.offset as usize..span.end.offset as usize]
        } else {
            return None;
        };

        if !func_name.eq_ignore_ascii_case("implode") && !func_name.eq_ignore_ascii_case("join") {
            return None;
        }

        let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
        if args.len() >= 2 {
            return Some(get_expr_text(args[1].value(), source));
        }
    }
    None
}

/// Try to simplify a ternary implode pattern
fn try_simplify_ternary_implode(cond: &Conditional<'_>, source: &str) -> Option<String> {
    // Must have explicit then branch
    let then_expr = cond.then.as_ref()?;

    // Condition must be $arr === []
    let binary = if let Expression::Binary(binary) = cond.condition {
        binary
    } else {
        return None;
    };

    if !matches!(binary.operator, BinaryOperator::Identical(_)) {
        return None;
    }

    // One side should be variable/expr, other should be []
    let (arr_expr, empty_array) = if is_empty_array(binary.rhs) {
        (binary.lhs, binary.rhs)
    } else if is_empty_array(binary.lhs) {
        (binary.rhs, binary.lhs)
    } else {
        return None;
    };

    // Verify empty array
    if !is_empty_array(empty_array) {
        return None;
    }

    // Then branch must be empty string
    if !is_empty_string(then_expr, source) {
        return None;
    }

    // Else branch must be implode/join with second arg matching arr_expr
    let implode_arg = get_implode_second_arg(cond.r#else, source)?;
    let arr_text = get_expr_text(arr_expr, source);

    if implode_arg != arr_text {
        return None;
    }

    // Return the implode call
    let else_span = cond.r#else.span();
    Some(source[else_span.start.offset as usize..else_span.end.offset as usize].to_string())
}

use crate::registry::{Category, Rule};

pub struct TernaryImplodeToImplodeRule;

impl Rule for TernaryImplodeToImplodeRule {
    fn name(&self) -> &'static str {
        "ternary_implode_to_implode"
    }

    fn description(&self) -> &'static str {
        "Simplify redundant ternary with implode"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_ternary_implode_to_implode(program, source)
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
        check_ternary_implode_to_implode(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php $arr === [] ? '' : implode(',', $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php implode(',', $arr);");
    }

    #[test]
    fn test_with_join() {
        let source = "<?php $values === [] ? '' : join('-', $values);";
        assert_eq!(transform(source), "<?php join('-', $values);");
    }

    #[test]
    fn test_double_quoted_empty() {
        let source = r#"<?php $arr === [] ? "" : implode(',', $arr);"#;
        assert_eq!(transform(source), "<?php implode(',', $arr);");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = $arr === [] ? '' : implode(',', $arr);";
        assert_eq!(transform(source), "<?php $result = implode(',', $arr);");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return $items === [] ? '' : implode(', ', $items);";
        assert_eq!(transform(source), "<?php return implode(', ', $items);");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = $x === [] ? '' : implode(',', $x);
$b = $y === [] ? '' : implode('-', $y);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_non_empty_string() {
        // Non-empty string in then branch
        let source = "<?php $arr === [] ? 'none' : implode(',', $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_different_array() {
        // Different arrays in condition and implode
        let source = "<?php $arr === [] ? '' : implode(',', $other);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_not_equal() {
        // !== instead of ===
        let source = "<?php $arr !== [] ? implode(',', $arr) : '';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php $arr === [] ? '' : count($arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
