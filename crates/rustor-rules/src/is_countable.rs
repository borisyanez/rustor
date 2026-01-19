//! Rule: Convert is_array() || instanceof Countable to is_countable() (PHP 7.3+)
//!
//! PHP 7.3 introduced is_countable() which checks if a value can be counted.
//!
//! Transformations:
//! - `is_array($x) || $x instanceof Countable` → `is_countable($x)`
//! - `$x instanceof Countable || is_array($x)` → `is_countable($x)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for is_array || instanceof Countable patterns
pub fn check_is_countable<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = IsCountableVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct IsCountableVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for IsCountableVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            if let Some(replacement) = try_transform_is_countable(binary, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Replace is_array() || instanceof Countable with is_countable() (PHP 7.3+)",
                ));
                return false;
            }
        }
        true
    }
}

/// Extract variable from is_array($var) call
fn extract_is_array_var<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if name.eq_ignore_ascii_case("is_array") {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
                if args.len() == 1 {
                    let arg_span = args[0].span();
                    return Some(
                        source[arg_span.start.offset as usize..arg_span.end.offset as usize]
                            .to_string(),
                    );
                }
            }
        }
    }
    None
}

/// Extract variable from $var instanceof Countable
fn extract_instanceof_countable_var<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Binary(binary) = expr {
        if let BinaryOperator::Instanceof(_) = &binary.operator {
            // Check if right side is "Countable"
            let right_span = binary.rhs.span();
            let right_text = &source[right_span.start.offset as usize..right_span.end.offset as usize];

            // Could be \Countable or Countable
            let class_name = right_text.trim_start_matches('\\');
            if class_name == "Countable" {
                let left_span = binary.lhs.span();
                return Some(
                    source[left_span.start.offset as usize..left_span.end.offset as usize]
                        .to_string(),
                );
            }
        }
    }
    None
}

/// Try to transform is_array || instanceof Countable pattern
fn try_transform_is_countable(binary: &Binary<'_>, source: &str) -> Option<String> {
    // Must be || operator
    if !matches!(binary.operator, BinaryOperator::Or(_)) {
        return None;
    }

    // Try both orders:
    // 1. is_array($x) || $x instanceof Countable
    // 2. $x instanceof Countable || is_array($x)

    let (is_array_var, instanceof_var) =
        if let Some(arr_var) = extract_is_array_var(binary.lhs, source) {
            let inst_var = extract_instanceof_countable_var(binary.rhs, source)?;
            (arr_var, inst_var)
        } else if let Some(inst_var) = extract_instanceof_countable_var(binary.lhs, source) {
            let arr_var = extract_is_array_var(binary.rhs, source)?;
            (arr_var, inst_var)
        } else {
            return None;
        };

    // Both must reference the same variable
    if is_array_var != instanceof_var {
        return None;
    }

    Some(format!("is_countable({})", is_array_var))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct IsCountableRule;

impl Rule for IsCountableRule {
    fn name(&self) -> &'static str {
        "is_countable"
    }

    fn description(&self) -> &'static str {
        "Replace is_array() || instanceof Countable with is_countable() (PHP 7.3+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_is_countable(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php73)
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
        check_is_countable(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_is_array_first() {
        let source = "<?php is_array($foo) || $foo instanceof Countable;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php is_countable($foo);");
    }

    #[test]
    fn test_instanceof_first() {
        let source = "<?php $foo instanceof Countable || is_array($foo);";
        assert_eq!(transform(source), "<?php is_countable($foo);");
    }

    #[test]
    fn test_with_backslash() {
        let source = r"<?php is_array($x) || $x instanceof \Countable;";
        assert_eq!(transform(source), "<?php is_countable($x);");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_condition() {
        let source = "<?php if (is_array($arr) || $arr instanceof Countable) {}";
        assert_eq!(transform(source), "<?php if (is_countable($arr)) {}");
    }

    #[test]
    fn test_in_assignment() {
        let source = "<?php $can_count = is_array($data) || $data instanceof Countable;";
        assert_eq!(transform(source), "<?php $can_count = is_countable($data);");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return is_array($val) || $val instanceof Countable;";
        assert_eq!(transform(source), "<?php return is_countable($val);");
    }

    // ==================== Complex Variables ====================

    #[test]
    fn test_property_access() {
        let source = "<?php is_array($obj->items) || $obj->items instanceof Countable;";
        assert_eq!(transform(source), "<?php is_countable($obj->items);");
    }

    #[test]
    fn test_array_access() {
        let source = "<?php is_array($arr[0]) || $arr[0] instanceof Countable;";
        assert_eq!(transform(source), "<?php is_countable($arr[0]);");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
is_array($a) || $a instanceof Countable;
is_array($b) || $b instanceof Countable;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_different_vars() {
        let source = "<?php is_array($foo) || $bar instanceof Countable;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_different_class() {
        let source = "<?php is_array($foo) || $foo instanceof Iterator;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_and_operator() {
        let source = "<?php is_array($foo) && $foo instanceof Countable;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_only_is_array() {
        let source = "<?php is_array($foo) || $bar;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_only_instanceof() {
        let source = "<?php $foo instanceof Countable || $bar;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
