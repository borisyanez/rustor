//! Rule: Convert `is_object($x) ? get_class($x) : gettype($x)` to `get_debug_type($x)`
//!
//! PHP 8.0 introduced `get_debug_type()` which returns a more useful type string.
//! This transformation simplifies the common ternary pattern used before PHP 8.0.
//!
//! Patterns handled:
//! - `is_object($x) ? get_class($x) : gettype($x)` → `get_debug_type($x)`
//! - `is_object($x) ? $x::class : gettype($x)` → `get_debug_type($x)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for ternary expressions that can use get_debug_type()
pub fn check_get_debug_type<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = GetDebugTypeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct GetDebugTypeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for GetDebugTypeVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Conditional(cond) = expr {
            if let Some(replacement) = try_transform_get_debug_type(cond, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Replace type-checking ternary with get_debug_type() (PHP 8.0+)",
                ));
                return false; // Don't traverse children
            }
        }
        true // Continue traversal
    }
}

/// Try to transform a conditional to get_debug_type(), returning the replacement if successful
fn try_transform_get_debug_type(cond: &Conditional<'_>, source: &str) -> Option<String> {
    // Must be a full ternary (not short ternary like $a ?: $b)
    let then_expr = cond.then.as_ref()?;

    // Step 1: Check condition is is_object($var)
    let condition_var = extract_is_object_var(cond.condition, source)?;

    // Step 2: Check then-branch is get_class($var) or $var::class
    let then_var = extract_get_class_or_class_const(then_expr, source)?;

    // Step 3: Check else-branch is gettype($var)
    let else_var = extract_gettype_var(cond.r#else, source)?;

    // Step 4: All variables must be identical
    if condition_var != then_var || condition_var != else_var {
        return None;
    }

    // All checks passed - create the replacement
    Some(format!("get_debug_type({})", condition_var))
}

/// Extract variable from is_object($var) call
fn extract_is_object_var<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name = get_span_text(ident.span(), source);
            if name.eq_ignore_ascii_case("is_object") {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
                if args.len() == 1 {
                    return Some(get_span_text(args[0].span(), source).to_string());
                }
            }
        }
    }
    None
}

/// Extract variable from get_class($var) or $var::class
fn extract_get_class_or_class_const<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    match expr {
        // get_class($var)
        Expression::Call(Call::Function(func_call)) => {
            if let Expression::Identifier(ident) = func_call.function {
                let name = get_span_text(ident.span(), source);
                if name.eq_ignore_ascii_case("get_class") {
                    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
                    if args.len() == 1 {
                        return Some(get_span_text(args[0].span(), source).to_string());
                    }
                }
            }
            None
        }
        // $var::class
        Expression::Access(Access::ClassConstant(cc)) => {
            // Check if accessing ::class constant
            let const_name = match &cc.constant {
                ClassLikeConstantSelector::Identifier(ident) => {
                    get_span_text(ident.span(), source)
                }
                _ => return None,
            };

            if const_name == "class" {
                // The class part should be a variable expression
                let class_text = get_span_text(cc.class.span(), source);
                // Only match variable expressions like $var
                if class_text.starts_with('$') {
                    return Some(class_text.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract variable from gettype($var) call
fn extract_gettype_var<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name = get_span_text(ident.span(), source);
            if name.eq_ignore_ascii_case("gettype") {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
                if args.len() == 1 {
                    return Some(get_span_text(args[0].span(), source).to_string());
                }
            }
        }
    }
    None
}

/// Helper to get source text from a span
fn get_span_text(span: mago_span::Span, source: &str) -> &str {
    &source[span.start.offset as usize..span.end.offset as usize]
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct GetDebugTypeRule;

impl Rule for GetDebugTypeRule {
    fn name(&self) -> &'static str {
        "get_debug_type"
    }

    fn description(&self) -> &'static str {
        "Convert is_object() ? get_class() : gettype() to get_debug_type() (PHP 8.0+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_get_debug_type(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
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
        check_get_debug_type(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_basic_pattern() {
        let source = "<?php is_object($value) ? get_class($value) : gettype($value);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php get_debug_type($value);");
    }

    #[test]
    fn test_pattern_in_return() {
        let source = "<?php return is_object($x) ? get_class($x) : gettype($x);";
        assert_eq!(transform(source), "<?php return get_debug_type($x);");
    }

    #[test]
    fn test_pattern_in_assignment() {
        let source = "<?php $type = is_object($obj) ? get_class($obj) : gettype($obj);";
        assert_eq!(
            transform(source),
            "<?php $type = get_debug_type($obj);"
        );
    }

    #[test]
    fn test_pattern_in_function_call() {
        let source = "<?php echo is_object($x) ? get_class($x) : gettype($x);";
        assert_eq!(transform(source), "<?php echo get_debug_type($x);");
    }

    // ==================== ::class Syntax Tests ====================

    #[test]
    fn test_class_const_syntax() {
        let source = "<?php is_object($value) ? $value::class : gettype($value);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php get_debug_type($value);");
    }

    #[test]
    fn test_class_const_in_return() {
        let source = "<?php return is_object($obj) ? $obj::class : gettype($obj);";
        assert_eq!(transform(source), "<?php return get_debug_type($obj);");
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_in_array() {
        let source = "<?php $arr = [is_object($x) ? get_class($x) : gettype($x)];";
        assert_eq!(transform(source), "<?php $arr = [get_debug_type($x)];");
    }

    #[test]
    fn test_in_string_interpolation_concat() {
        let source = r#"<?php echo "Type: " . (is_object($v) ? get_class($v) : gettype($v));"#;
        // The parentheses around the ternary are preserved since we only replace the ternary itself
        assert_eq!(
            transform(source),
            r#"<?php echo "Type: " . (get_debug_type($v));"#
        );
    }

    #[test]
    fn test_in_method() {
        let source = r#"<?php
class Foo {
    public function getType($val) {
        return is_object($val) ? get_class($val) : gettype($val);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_functions() {
        let source = "<?php IS_OBJECT($x) ? GET_CLASS($x) : GETTYPE($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_case() {
        let source = "<?php Is_Object($x) ? Get_Class($x) : GetType($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases - Different Variables ====================

    #[test]
    fn test_skip_different_vars_condition_if() {
        // Condition uses $x, if uses $y
        let source = "<?php is_object($x) ? get_class($y) : gettype($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_different_vars_if_else() {
        // If uses $x, else uses $y
        let source = "<?php is_object($x) ? get_class($x) : gettype($y);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_all_different_vars() {
        let source = "<?php is_object($a) ? get_class($b) : gettype($c);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Skip Cases - Wrong Functions ====================

    #[test]
    fn test_skip_wrong_condition_function() {
        let source = "<?php is_array($x) ? get_class($x) : gettype($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_wrong_if_function() {
        let source = "<?php is_object($x) ? typeof($x) : gettype($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_wrong_else_function() {
        let source = "<?php is_object($x) ? get_class($x) : typeof($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Skip Cases - Wrong Structure ====================

    #[test]
    fn test_skip_short_ternary() {
        // Short ternary doesn't have explicit if-branch
        let source = "<?php is_object($x) ?: gettype($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_ternary() {
        let source = "<?php get_class($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_regular_ternary() {
        // Just a regular ternary, not the pattern we're looking for
        let source = "<?php $x > 0 ? 'positive' : 'non-positive';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Complex Variable Expressions ====================

    #[test]
    fn test_array_access_var() {
        let source = "<?php is_object($arr[0]) ? get_class($arr[0]) : gettype($arr[0]);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php get_debug_type($arr[0]);");
    }

    #[test]
    fn test_property_access_var() {
        let source = "<?php is_object($obj->prop) ? get_class($obj->prop) : gettype($obj->prop);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php get_debug_type($obj->prop);");
    }

    #[test]
    fn test_method_call_var() {
        let source = "<?php is_object($obj->get()) ? get_class($obj->get()) : gettype($obj->get());";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_patterns() {
        let source = r#"<?php
$a = is_object($x) ? get_class($x) : gettype($x);
$b = is_object($y) ? get_class($y) : gettype($y);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_with_whitespace() {
        let source = "<?php is_object( $x ) ? get_class( $x ) : gettype( $x );";
        let edits = check_php(source);
        // The extracted variable is just "$x" (no whitespace) from all three positions
        // so they all match and the rule applies
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php get_debug_type($x);");
    }

    #[test]
    fn test_parenthesized_ternary() {
        let source = "<?php (is_object($x) ? get_class($x) : gettype($x));";
        let edits = check_php(source);
        // The ternary itself should still match
        assert_eq!(edits.len(), 1);
    }
}
