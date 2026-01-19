//! Rule: Convert array_key_exists ternary to null coalescing
//!
//! When checking if a key exists and returning the value or null,
//! use the null coalescing operator instead.
//!
//! Transformation:
//! - `array_key_exists($key, $arr) ? $arr[$key] : null` → `$arr[$key] ?? null`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for array_key_exists ternary patterns
pub fn check_array_key_exists_to_coalesce<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ArrayKeyExistsToCoalesceVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ArrayKeyExistsToCoalesceVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ArrayKeyExistsToCoalesceVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Conditional(cond) = expr {
            if let Some(replacement) = try_transform_array_key_exists_ternary(cond, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Convert array_key_exists ternary to null coalescing",
                ));
                return false;
            }
        }
        true
    }
}

/// Check if expression is null literal
fn is_null_literal(expr: &Expression<'_>, source: &str) -> bool {
    if let Expression::Literal(Literal::Null(_)) = expr {
        return true;
    }
    // Also check for identifier null (case insensitive)
    if let Expression::Identifier(ident) = expr {
        let span = ident.span();
        let name = &source[span.start.offset as usize..span.end.offset as usize];
        return name.eq_ignore_ascii_case("null");
    }
    false
}

/// Check if expression is array access $arr[$key]
fn get_array_access_info<'a>(expr: &'a Expression<'a>, source: &str) -> Option<(String, String)> {
    if let Expression::ArrayAccess(array_access) = expr {
        let array_span = array_access.array.span();
        let array_text = source[array_span.start.offset as usize..array_span.end.offset as usize].to_string();

        let index_span = array_access.index.span();
        let index_text = source[index_span.start.offset as usize..index_span.end.offset as usize].to_string();
        return Some((array_text, index_text));
    }
    None
}

/// Try to transform array_key_exists ternary, returning the replacement if successful
fn try_transform_array_key_exists_ternary(cond: &Conditional<'_>, source: &str) -> Option<String> {
    // Must have explicit then branch
    let then_expr = cond.then.as_ref()?;

    // Condition must be array_key_exists($key, $arr)
    let func_call = if let Expression::Call(Call::Function(fc)) = cond.condition {
        fc
    } else {
        return None;
    };

    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("array_key_exists") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // array_key_exists needs exactly 2 arguments
    if args.len() != 2 {
        return None;
    }

    // Get key and array from array_key_exists args
    let key_span = args[0].span();
    let key_text = &source[key_span.start.offset as usize..key_span.end.offset as usize];

    let arr_span = args[1].span();
    let arr_text = &source[arr_span.start.offset as usize..arr_span.end.offset as usize];

    // Then branch must be $arr[$key]
    let (then_array, then_key) = get_array_access_info(then_expr, source)?;

    // Verify array and key match
    if then_array != arr_text || then_key != key_text {
        return None;
    }

    // Else branch must be null
    if !is_null_literal(cond.r#else, source) {
        return None;
    }

    // Build replacement: $arr[$key] ?? null
    Some(format!("{}[{}] ?? null", arr_text, key_text))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct ArrayKeyExistsToCoalesceRule;

impl Rule for ArrayKeyExistsToCoalesceRule {
    fn name(&self) -> &'static str {
        "array_key_exists_to_coalesce"
    }

    fn description(&self) -> &'static str {
        "Convert array_key_exists ternary to null coalescing operator"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_array_key_exists_to_coalesce(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php70)
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
        check_array_key_exists_to_coalesce(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php array_key_exists($key, $arr) ? $arr[$key] : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $arr[$key] ?? null;");
    }

    #[test]
    fn test_string_key() {
        let source = "<?php array_key_exists('name', $data) ? $data['name'] : null;";
        assert_eq!(transform(source), "<?php $data['name'] ?? null;");
    }

    #[test]
    fn test_variable_array() {
        let source = "<?php array_key_exists($k, $values) ? $values[$k] : null;";
        assert_eq!(transform(source), "<?php $values[$k] ?? null;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = array_key_exists($key, $arr) ? $arr[$key] : null;";
        assert_eq!(transform(source), "<?php $result = $arr[$key] ?? null;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return array_key_exists('id', $row) ? $row['id'] : null;";
        assert_eq!(transform(source), "<?php return $row['id'] ?? null;");
    }

    // ==================== Case Insensitive ====================

    #[test]
    fn test_uppercase() {
        let source = "<?php ARRAY_KEY_EXISTS($k, $a) ? $a[$k] : NULL;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = array_key_exists($k, $x) ? $x[$k] : null;
$b = array_key_exists($j, $y) ? $y[$j] : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_different_array() {
        let source = "<?php array_key_exists($key, $arr) ? $other[$key] : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_different_key() {
        let source = "<?php array_key_exists($key, $arr) ? $arr[$other] : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_null_else() {
        let source = "<?php array_key_exists($key, $arr) ? $arr[$key] : 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_isset() {
        // isset is different - only transform array_key_exists
        let source = "<?php isset($arr[$key]) ? $arr[$key] : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_single_arg() {
        let source = "<?php array_key_exists($key) ? $arr[$key] : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
