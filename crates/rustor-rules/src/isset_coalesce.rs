//! Rule: Convert isset ternary to null coalescing operator
//!
//! - isset($x) ? $x : $default → $x ?? $default
//! - isset($arr['key']) ? $arr['key'] : $default → $arr['key'] ?? $default
//!
//! The null coalescing operator (??) was introduced in PHP 7 and is more
//! concise than the isset ternary pattern.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for isset ternary patterns that can use null coalescing
pub fn check_isset_coalesce<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = IssetCoalesceVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct IssetCoalesceVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for IssetCoalesceVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_isset_coalesce(expr, self.source) {
            self.edits.push(edit);
            return false; // Don't traverse children
        }
        true // Continue traversal
    }
}

/// Try to transform an isset ternary, returning the Edit if successful
fn try_transform_isset_coalesce(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    // Match ternary expression: condition ? then : else
    let ternary = match expr {
        Expression::Conditional(t) => t,
        _ => return None,
    };

    // The "then" part must exist (not short ternary ?:)
    let then_expr = ternary.then.as_ref()?;

    // Check if condition is isset() construct (isset is a language construct, not a function)
    let isset_arg_code = match &*ternary.condition {
        Expression::Construct(Construct::Isset(isset)) => {
            // Only handle single-argument isset
            let values: Vec<_> = isset.values.iter().collect();
            if values.len() != 1 {
                return None;
            }

            let arg_span = values[0].span();
            &source[arg_span.start.offset as usize..arg_span.end.offset as usize]
        }
        _ => return None,
    };

    // Check if "then" expression matches the isset argument
    let then_span = then_expr.span();
    let then_code = &source[then_span.start.offset as usize..then_span.end.offset as usize];

    // The then expression must be the same as the isset argument
    if then_code != isset_arg_code {
        return None;
    }

    // Get the else expression
    let else_span = ternary.r#else.span();
    let else_code = &source[else_span.start.offset as usize..else_span.end.offset as usize];

    // Build replacement: $x ?? $default
    let replacement = format!("{} ?? {}", isset_arg_code, else_code);

    Some(Edit::new(
        expr.span(),
        replacement,
        "Replace isset() ternary with ?? (null coalescing operator)",
    ))
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
        check_isset_coalesce(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_isset_ternary() {
        let source = "<?php isset($x) ? $x : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x ?? null;");
    }

    #[test]
    fn test_isset_with_default_value() {
        let source = "<?php isset($x) ? $x : 'default';";
        assert_eq!(transform(source), "<?php $x ?? 'default';");
    }

    #[test]
    fn test_isset_with_variable_default() {
        let source = "<?php isset($x) ? $x : $default;";
        assert_eq!(transform(source), "<?php $x ?? $default;");
    }

    #[test]
    fn test_isset_in_assignment() {
        let source = "<?php $val = isset($x) ? $x : 'fallback';";
        assert_eq!(transform(source), "<?php $val = $x ?? 'fallback';");
    }

    #[test]
    fn test_isset_with_array_access() {
        let source = "<?php isset($arr['key']) ? $arr['key'] : null;";
        assert_eq!(transform(source), "<?php $arr['key'] ?? null;");
    }

    #[test]
    fn test_isset_with_nested_array() {
        let source = "<?php isset($arr['a']['b']) ? $arr['a']['b'] : 0;";
        assert_eq!(transform(source), "<?php $arr['a']['b'] ?? 0;");
    }

    #[test]
    fn test_isset_with_object_property() {
        let source = "<?php isset($obj->prop) ? $obj->prop : false;";
        assert_eq!(transform(source), "<?php $obj->prop ?? false;");
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_isset_ternaries() {
        let source = "<?php $a = isset($x) ? $x : 1; $b = isset($y) ? $y : 2;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a = $x ?? 1; $b = $y ?? 2;");
    }

    #[test]
    fn test_isset_in_expression() {
        let source = "<?php $result = (isset($a) ? $a : 0) + (isset($b) ? $b : 0);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $result = ($a ?? 0) + ($b ?? 0);");
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_isset_in_function() {
        let source = r#"<?php
function getValue($arr) {
    return isset($arr['key']) ? $arr['key'] : 'default';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_isset_in_class_method() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return isset($this->data) ? $this->data : [];
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_isset_in_if_condition() {
        let source = "<?php if ((isset($x) ? $x : 0) > 5) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_isset_in_array() {
        let source = "<?php $arr = [isset($a) ? $a : 1, isset($b) ? $b : 2];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_isset() {
        let source = "<?php ISSET($x) ? $x : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_case_isset() {
        let source = "<?php IsSet($x) ? $x : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_mismatched_variable() {
        // $x in isset but $y in then - don't transform
        let source = "<?php isset($x) ? $y : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_multi_arg_isset() {
        // isset with multiple arguments - don't transform
        let source = "<?php isset($x, $y) ? $x : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_short_ternary() {
        // Short ternary ?: is different semantics
        let source = "<?php isset($x) ?: 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_different_expression() {
        // Complex expression in then part
        let source = "<?php isset($x) ? $x + 1 : 0;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_isset_condition() {
        let source = "<?php $x ? $x : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_empty_function() {
        let source = "<?php empty($x) ? null : $x;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_isset_with_numeric_default() {
        let source = "<?php isset($count) ? $count : 0;";
        assert_eq!(transform(source), "<?php $count ?? 0;");
    }

    #[test]
    fn test_isset_with_array_default() {
        let source = "<?php isset($items) ? $items : [];";
        assert_eq!(transform(source), "<?php $items ?? [];");
    }

    #[test]
    fn test_isset_with_function_call_default() {
        let source = "<?php isset($val) ? $val : getDefault();";
        assert_eq!(transform(source), "<?php $val ?? getDefault();");
    }

    #[test]
    fn test_deeply_nested() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($condition) {
            foreach ($items as $item) {
                $val = isset($item['key']) ? $item['key'] : 'none';
            }
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_nested_isset_coalesce() {
        // Outer ternary is transformed; inner one remains (would need second pass)
        let source = "<?php isset($a) ? $a : (isset($b) ? $b : 'default');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php $a ?? (isset($b) ? $b : 'default');"
        );
    }
}
