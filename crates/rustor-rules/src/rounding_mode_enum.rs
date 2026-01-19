//! Rule: Replace rounding mode constant with RoundingMode enum
//!
//! Since PHP 8.4, round() supports RoundingMode enum instead of constants.
//!
//! Transformations:
//! - `round($x, 0, PHP_ROUND_HALF_UP)` → `round($x, 0, \RoundingMode::HalfAwayFromZero)`
//! - `round($x, 0, PHP_ROUND_HALF_DOWN)` → `round($x, 0, \RoundingMode::HalfTowardsZero)`
//! - `round($x, 0, PHP_ROUND_HALF_EVEN)` → `round($x, 0, \RoundingMode::HalfEven)`
//! - `round($x, 0, PHP_ROUND_HALF_ODD)` → `round($x, 0, \RoundingMode::HalfOdd)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for round() with rounding mode constants
pub fn check_rounding_mode_enum<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RoundingModeEnumVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RoundingModeEnumVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RoundingModeEnumVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_replace_rounding_mode(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to replace rounding mode constant with enum
fn try_replace_rounding_mode(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name is "round"
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("round") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Must have 3 arguments (value, precision, mode)
    if args.len() != 3 {
        return None;
    }

    // Check if the third argument is a constant access (rounding mode constant)
    let mode_arg = args[2].value();
    let const_name = if let Expression::ConstantAccess(const_access) = mode_arg {
        let span = const_access.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    // Map constant to enum case
    let enum_case = match const_name {
        "PHP_ROUND_HALF_UP" => "HalfAwayFromZero",
        "PHP_ROUND_HALF_DOWN" => "HalfTowardsZero",
        "PHP_ROUND_HALF_EVEN" => "HalfEven",
        "PHP_ROUND_HALF_ODD" => "HalfOdd",
        _ => return None,
    };

    // Replace just the constant with the enum
    let mode_span = mode_arg.span();
    Some(Edit::new(
        mode_span,
        format!("\\RoundingMode::{}", enum_case),
        "Replace rounding mode constant with RoundingMode enum",
    ))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct RoundingModeEnumRule;

impl Rule for RoundingModeEnumRule {
    fn name(&self) -> &'static str {
        "rounding_mode_enum"
    }

    fn description(&self) -> &'static str {
        "Replace rounding mode constant with RoundingMode enum"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_rounding_mode_enum(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php84)
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
        check_rounding_mode_enum(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== PHP_ROUND_HALF_UP ====================

    #[test]
    fn test_half_up() {
        let source = "<?php round(1.5, 0, PHP_ROUND_HALF_UP);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            r"<?php round(1.5, 0, \RoundingMode::HalfAwayFromZero);"
        );
    }

    // ==================== PHP_ROUND_HALF_DOWN ====================

    #[test]
    fn test_half_down() {
        let source = "<?php round(1.5, 0, PHP_ROUND_HALF_DOWN);";
        assert_eq!(
            transform(source),
            r"<?php round(1.5, 0, \RoundingMode::HalfTowardsZero);"
        );
    }

    // ==================== PHP_ROUND_HALF_EVEN ====================

    #[test]
    fn test_half_even() {
        let source = "<?php round(1.5, 0, PHP_ROUND_HALF_EVEN);";
        assert_eq!(
            transform(source),
            r"<?php round(1.5, 0, \RoundingMode::HalfEven);"
        );
    }

    // ==================== PHP_ROUND_HALF_ODD ====================

    #[test]
    fn test_half_odd() {
        let source = "<?php round(1.5, 0, PHP_ROUND_HALF_ODD);";
        assert_eq!(
            transform(source),
            r"<?php round(1.5, 0, \RoundingMode::HalfOdd);"
        );
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = round($value, 2, PHP_ROUND_HALF_UP);";
        assert_eq!(
            transform(source),
            r"<?php $result = round($value, 2, \RoundingMode::HalfAwayFromZero);"
        );
    }

    #[test]
    fn test_with_variables() {
        let source = "<?php round($num, $precision, PHP_ROUND_HALF_EVEN);";
        assert_eq!(
            transform(source),
            r"<?php round($num, $precision, \RoundingMode::HalfEven);"
        );
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = round(1.5, 0, PHP_ROUND_HALF_UP);
$b = round(2.5, 0, PHP_ROUND_HALF_DOWN);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_two_args() {
        // round() with only 2 args should not be transformed
        let source = "<?php round(1.5, 0);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_one_arg() {
        let source = "<?php round(1.5);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_variable_mode() {
        // Third arg is a variable, not a constant
        let source = "<?php round(1.5, 0, $mode);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_constant() {
        // Unknown constant
        let source = "<?php round(1.5, 0, SOME_OTHER_CONSTANT);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_already_enum() {
        // Already using enum (this won't match our pattern anyway)
        let source = r"<?php round(1.5, 0, \RoundingMode::HalfAwayFromZero);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php floor(1.5, 0, PHP_ROUND_HALF_UP);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
