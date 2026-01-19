//! Rule: Convert version_compare(PHP_VERSION, ...) to PHP_VERSION_ID comparison
//!
//! Using PHP_VERSION_ID constant is faster than calling version_compare with PHP_VERSION.
//!
//! Transformations:
//! - `version_compare(PHP_VERSION, '5.3.0', '<')` → `PHP_VERSION_ID < 50300`
//! - `version_compare(PHP_VERSION, '7.4.0', '>=')` → `PHP_VERSION_ID >= 70400`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for version_compare with PHP_VERSION
pub fn check_version_compare_to_constant<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = VersionCompareVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct VersionCompareVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for VersionCompareVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_convert_version_compare(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Check if expression is a PHP_VERSION constant
fn is_php_version_constant(expr: &Expression<'_>, source: &str) -> bool {
    if let Expression::ConstantAccess(const_access) = expr {
        let span = const_access.span();
        let text = &source[span.start.offset as usize..span.end.offset as usize];
        return text == "PHP_VERSION";
    }
    false
}

/// Parse a version string like "5.3.0" or "7.4" into PHP_VERSION_ID format
fn parse_version_string(version: &str) -> Option<u32> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }

    let major: u32 = parts.first()?.parse().ok()?;
    let minor: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    // PHP_VERSION_ID = major * 10000 + minor * 100 + patch
    Some(major * 10000 + minor * 100 + patch)
}

/// Get string value from a string literal
fn get_string_value(expr: &Expression<'_>, source: &str) -> Option<String> {
    if let Expression::Literal(Literal::String(string_lit)) = expr {
        let span = string_lit.span();
        let raw = &source[span.start.offset as usize..span.end.offset as usize];

        if raw.starts_with('\'') && raw.ends_with('\'') {
            return Some(raw[1..raw.len() - 1].to_string());
        } else if raw.starts_with('"') && raw.ends_with('"') {
            return Some(raw[1..raw.len() - 1].to_string());
        }
    }
    None
}

/// Map operator string to comparison operator
fn get_operator_symbol(op: &str) -> Option<&'static str> {
    match op {
        "=" | "==" | "eq" => Some("==="),
        "!=" | "<>" | "ne" => Some("!=="),
        ">" | "gt" => Some(">"),
        "<" | "lt" => Some("<"),
        ">=" | "ge" => Some(">="),
        "<=" | "le" => Some("<="),
        _ => None,
    }
}

/// Try to convert version_compare(PHP_VERSION, '...', op) to PHP_VERSION_ID comparison
fn try_convert_version_compare(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name is "version_compare"
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("version_compare") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Must have exactly 3 arguments
    if args.len() != 3 {
        return None;
    }

    let arg0 = args[0].value();
    let arg1 = args[1].value();
    let arg2 = args[2].value();

    // Determine which argument is PHP_VERSION and which is the version string
    let (version_id_left, version_str) = if is_php_version_constant(arg0, source) {
        // version_compare(PHP_VERSION, '5.3.0', op)
        (true, get_string_value(arg1, source)?)
    } else if is_php_version_constant(arg1, source) {
        // version_compare('5.3.0', PHP_VERSION, op)
        (false, get_string_value(arg0, source)?)
    } else {
        return None;
    };

    // Parse the version string to version ID
    let version_id = parse_version_string(&version_str)?;

    // Get the operator
    let op_str = get_string_value(arg2, source)?;
    let comparison_op = get_operator_symbol(&op_str)?;

    // Build the replacement
    let func_span = func_call.span();
    let replacement = if version_id_left {
        format!("PHP_VERSION_ID {} {}", comparison_op, version_id)
    } else {
        // If version string was on the left, swap the comparison
        let swapped_op = match comparison_op {
            "<" => ">",
            ">" => "<",
            "<=" => ">=",
            ">=" => "<=",
            other => other, // === and !== are symmetric
        };
        format!("PHP_VERSION_ID {} {}", swapped_op, version_id)
    };

    Some(Edit::new(
        func_span,
        replacement,
        "Convert version_compare to PHP_VERSION_ID comparison",
    ))
}

use crate::registry::{Category, Rule};

pub struct VersionCompareToConstantRule;

impl Rule for VersionCompareToConstantRule {
    fn name(&self) -> &'static str {
        "version_compare_to_constant"
    }

    fn description(&self) -> &'static str {
        "Convert version_compare(PHP_VERSION, ...) to PHP_VERSION_ID comparison"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_version_compare_to_constant(program, source)
    }

    fn category(&self) -> Category {
        Category::Performance
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
        check_version_compare_to_constant(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Operators ====================

    #[test]
    fn test_less_than() {
        let source = "<?php version_compare(PHP_VERSION, '5.3.0', '<');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php PHP_VERSION_ID < 50300;");
    }

    #[test]
    fn test_greater_or_equal() {
        let source = "<?php version_compare(PHP_VERSION, '7.4.0', '>=');";
        assert_eq!(transform(source), "<?php PHP_VERSION_ID >= 70400;");
    }

    #[test]
    fn test_greater_than() {
        let source = "<?php version_compare(PHP_VERSION, '8.0.0', '>');";
        assert_eq!(transform(source), "<?php PHP_VERSION_ID > 80000;");
    }

    #[test]
    fn test_less_or_equal() {
        let source = "<?php version_compare(PHP_VERSION, '7.2.0', '<=');";
        assert_eq!(transform(source), "<?php PHP_VERSION_ID <= 70200;");
    }

    #[test]
    fn test_equal() {
        let source = "<?php version_compare(PHP_VERSION, '7.4.0', '==');";
        assert_eq!(transform(source), "<?php PHP_VERSION_ID === 70400;");
    }

    #[test]
    fn test_not_equal() {
        let source = "<?php version_compare(PHP_VERSION, '5.6.0', '!=');";
        assert_eq!(transform(source), "<?php PHP_VERSION_ID !== 50600;");
    }

    // ==================== Alternative Operators ====================

    #[test]
    fn test_op_lt() {
        let source = "<?php version_compare(PHP_VERSION, '7.0.0', 'lt');";
        assert_eq!(transform(source), "<?php PHP_VERSION_ID < 70000;");
    }

    #[test]
    fn test_op_ge() {
        let source = "<?php version_compare(PHP_VERSION, '8.1.0', 'ge');";
        assert_eq!(transform(source), "<?php PHP_VERSION_ID >= 80100;");
    }

    // ==================== Version Formats ====================

    #[test]
    fn test_two_part_version() {
        let source = "<?php version_compare(PHP_VERSION, '7.4', '>=');";
        assert_eq!(transform(source), "<?php PHP_VERSION_ID >= 70400;");
    }

    #[test]
    fn test_single_part_version() {
        let source = "<?php version_compare(PHP_VERSION, '8', '>=');";
        assert_eq!(transform(source), "<?php PHP_VERSION_ID >= 80000;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_condition() {
        let source = "<?php if (version_compare(PHP_VERSION, '7.4.0', '>=')) {}";
        assert_eq!(transform(source), "<?php if (PHP_VERSION_ID >= 70400) {}");
    }

    #[test]
    fn test_in_assignment() {
        let source = "<?php $is_php8 = version_compare(PHP_VERSION, '8.0.0', '>=');";
        assert_eq!(transform(source), "<?php $is_php8 = PHP_VERSION_ID >= 80000;");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = version_compare(PHP_VERSION, '7.0.0', '>=');
$b = version_compare(PHP_VERSION, '8.0.0', '<');
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_two_args() {
        // Without operator returns -1, 0, or 1
        let source = "<?php version_compare(PHP_VERSION, '7.4.0');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_php_version() {
        let source = "<?php version_compare($a, '7.4.0', '<');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_both_strings() {
        let source = "<?php version_compare('7.0.0', '7.4.0', '<');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php version_check(PHP_VERSION, '7.4.0', '<');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
