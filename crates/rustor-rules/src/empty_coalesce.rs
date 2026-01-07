//! Rule: Convert empty ternary to Elvis operator
//!
//! - empty($x) ? $default : $x → $x ?: $default
//! - !empty($x) ? $x : $default → $x ?: $default
//!
//! The Elvis operator (?:) is more concise for these common patterns.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for empty ternary patterns that can use Elvis operator
pub fn check_empty_coalesce<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = EmptyCoalesceVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct EmptyCoalesceVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for EmptyCoalesceVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_empty_coalesce(expr, self.source) {
            self.edits.push(edit);
            return false; // Don't traverse children
        }
        true // Continue traversal
    }
}

/// Extract the empty() argument code if expr is an empty() construct
fn get_empty_arg<'a>(expr: &Expression<'_>, source: &'a str) -> Option<&'a str> {
    if let Expression::Construct(Construct::Empty(empty)) = expr {
        let arg_span = empty.value.span();
        Some(&source[arg_span.start.offset as usize..arg_span.end.offset as usize])
    } else {
        None
    }
}

/// Try to transform an empty ternary, returning the Edit if successful
fn try_transform_empty_coalesce(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    // Match ternary expression: condition ? then : else
    let ternary = match expr {
        Expression::Conditional(t) => t,
        _ => return None,
    };

    // The "then" part must exist (not already short ternary ?:)
    let then_expr = ternary.then.as_ref()?;

    let then_span = then_expr.span();
    let then_code = &source[then_span.start.offset as usize..then_span.end.offset as usize];

    let else_span = ternary.r#else.span();
    let else_code = &source[else_span.start.offset as usize..else_span.end.offset as usize];

    // Pattern 1: empty($x) ? $default : $x → $x ?: $default
    // Condition is empty(), else matches the argument
    if let Some(empty_arg) = get_empty_arg(&ternary.condition, source) {
        if else_code == empty_arg {
            let replacement = format!("{} ?: {}", empty_arg, then_code);
            return Some(Edit::new(
                expr.span(),
                replacement,
                "Replace empty() ternary with ?: (Elvis operator)",
            ));
        }
    }

    // Pattern 2: !empty($x) ? $x : $default → $x ?: $default
    // Condition is !empty(), then matches the argument
    if let Expression::UnaryPrefix(unary) = &*ternary.condition {
        if let UnaryPrefixOperator::Not(_) = &unary.operator {
            if let Some(empty_arg) = get_empty_arg(&unary.operand, source) {
                if then_code == empty_arg {
                    let replacement = format!("{} ?: {}", empty_arg, else_code);
                    return Some(Edit::new(
                        expr.span(),
                        replacement,
                        "Replace !empty() ternary with ?: (Elvis operator)",
                    ));
                }
            }
        }
    }

    None
}

// Rule trait implementation
use crate::registry::{Category, Rule};

pub struct EmptyCoalesceRule;

impl Rule for EmptyCoalesceRule {
    fn name(&self) -> &'static str {
        "empty_coalesce"
    }

    fn description(&self) -> &'static str {
        "Convert empty($x) ? $default : $x to $x ?: $default"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_empty_coalesce(program, source)
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
        check_empty_coalesce(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Pattern 1: empty($x) ? $default : $x ====================

    #[test]
    fn test_empty_ternary_simple() {
        let source = "<?php empty($x) ? 'default' : $x;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x ?: 'default';");
    }

    #[test]
    fn test_empty_ternary_with_null() {
        let source = "<?php empty($val) ? null : $val;";
        assert_eq!(transform(source), "<?php $val ?: null;");
    }

    #[test]
    fn test_empty_ternary_in_assignment() {
        let source = "<?php $result = empty($x) ? 0 : $x;";
        assert_eq!(transform(source), "<?php $result = $x ?: 0;");
    }

    #[test]
    fn test_empty_ternary_with_array_access() {
        let source = "<?php empty($arr['key']) ? 'none' : $arr['key'];";
        assert_eq!(transform(source), "<?php $arr['key'] ?: 'none';");
    }

    #[test]
    fn test_empty_ternary_with_object_property() {
        let source = "<?php empty($obj->name) ? 'Anonymous' : $obj->name;";
        assert_eq!(transform(source), "<?php $obj->name ?: 'Anonymous';");
    }

    // ==================== Pattern 2: !empty($x) ? $x : $default ====================

    #[test]
    fn test_not_empty_ternary_simple() {
        let source = "<?php !empty($x) ? $x : 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x ?: 'default';");
    }

    #[test]
    fn test_not_empty_ternary_with_null() {
        let source = "<?php !empty($val) ? $val : null;";
        assert_eq!(transform(source), "<?php $val ?: null;");
    }

    #[test]
    fn test_not_empty_ternary_in_assignment() {
        let source = "<?php $result = !empty($x) ? $x : 0;";
        assert_eq!(transform(source), "<?php $result = $x ?: 0;");
    }

    #[test]
    fn test_not_empty_ternary_with_array_access() {
        let source = "<?php !empty($arr['key']) ? $arr['key'] : 'none';";
        assert_eq!(transform(source), "<?php $arr['key'] ?: 'none';");
    }

    #[test]
    fn test_not_empty_ternary_with_object_property() {
        let source = "<?php !empty($obj->name) ? $obj->name : 'Anonymous';";
        assert_eq!(transform(source), "<?php $obj->name ?: 'Anonymous';");
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_empty_ternaries() {
        let source = "<?php $a = empty($x) ? 1 : $x; $b = !empty($y) ? $y : 2;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a = $x ?: 1; $b = $y ?: 2;");
    }

    #[test]
    fn test_empty_in_expression() {
        let source = "<?php $result = (empty($a) ? 0 : $a) + (!empty($b) ? $b : 0);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $result = ($a ?: 0) + ($b ?: 0);");
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_empty_in_function() {
        let source = r#"<?php
function getValue($arr) {
    return empty($arr['key']) ? 'default' : $arr['key'];
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_empty_in_class_method() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return !empty($this->data) ? $this->data : [];
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_empty_in_if_condition() {
        let source = "<?php if ((empty($x) ? 0 : $x) > 5) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_empty_in_array() {
        let source = "<?php $arr = [empty($a) ? 1 : $a, !empty($b) ? $b : 2];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_empty() {
        let source = "<?php EMPTY($x) ? 'default' : $x;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_case_empty() {
        let source = "<?php Empty($x) ? 'default' : $x;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_mismatched_variable_pattern1() {
        // empty($x) but else is $y - don't transform
        let source = "<?php empty($x) ? 'default' : $y;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_mismatched_variable_pattern2() {
        // !empty($x) but then is $y - don't transform
        let source = "<?php !empty($x) ? $y : 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_already_elvis() {
        // Already using Elvis operator
        let source = "<?php $x ?: 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_isset_not_empty() {
        let source = "<?php isset($x) ? $x : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_wrong_pattern_order() {
        // empty($x) ? $x : 'default' - this is NOT the same as Elvis
        // (if empty, use the empty value, otherwise use default - makes no sense)
        let source = "<?php empty($x) ? $x : 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_not_empty_wrong_order() {
        // !empty($x) ? 'default' : $x - wrong order for Elvis
        let source = "<?php !empty($x) ? 'default' : $x;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_empty_with_numeric_default() {
        let source = "<?php empty($count) ? 0 : $count;";
        assert_eq!(transform(source), "<?php $count ?: 0;");
    }

    #[test]
    fn test_empty_with_array_default() {
        let source = "<?php empty($items) ? [] : $items;";
        assert_eq!(transform(source), "<?php $items ?: [];");
    }

    #[test]
    fn test_empty_with_function_call_default() {
        let source = "<?php empty($val) ? getDefault() : $val;";
        assert_eq!(transform(source), "<?php $val ?: getDefault();");
    }

    #[test]
    fn test_deeply_nested() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($condition) {
            foreach ($items as $item) {
                $val = empty($item['key']) ? 'none' : $item['key'];
            }
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_nested_empty_coalesce() {
        // Outer ternary is transformed; inner one remains
        let source = "<?php empty($a) ? (empty($b) ? 'x' : $b) : $a;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php $a ?: (empty($b) ? 'x' : $b);"
        );
    }
}
