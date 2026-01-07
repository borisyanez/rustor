//! Rule: Convert legacy array() syntax to short [] syntax
//!
//! - array() → []
//! - array(1, 2, 3) → [1, 2, 3]
//! - array('a' => 1) → ['a' => 1]
//!
//! Short array syntax was introduced in PHP 5.4 and is now the preferred style.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for legacy array() syntax
pub fn check_array_syntax<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ArraySyntaxVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ArraySyntaxVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ArraySyntaxVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_array_syntax(expr, self.source) {
            self.edits.push(edit);
            // Don't traverse into this array's children - they'll be in the replacement
            // and would cause overlapping edits. Nested arrays need another pass.
            return false;
        }
        true // Continue traversal
    }
}

/// Try to transform a legacy array, returning the Edit if successful
fn try_transform_array_syntax(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    if let Expression::LegacyArray(legacy_arr) = expr {
        let full_span = expr.span();

        // Get the content between parentheses
        let left_paren_end = legacy_arr.left_parenthesis.end.offset as usize;
        let right_paren_start = legacy_arr.right_parenthesis.start.offset as usize;
        let inner_content = &source[left_paren_end..right_paren_start];

        // Build replacement: [content]
        let replacement = format!("[{}]", inner_content);

        return Some(Edit::new(
            full_span,
            replacement,
            "Replace array() with [] (short array syntax)",
        ));
    }
    None
}

// Rule trait implementation
use crate::registry::Rule;

pub struct ArraySyntaxRule;

impl Rule for ArraySyntaxRule {
    fn name(&self) -> &'static str {
        "array_syntax"
    }

    fn description(&self) -> &'static str {
        "Convert array() to [] (short array syntax)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_array_syntax(program, source)
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
        check_array_syntax(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_empty_array() {
        let source = "<?php array();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php [];");
    }

    #[test]
    fn test_simple_array() {
        let source = "<?php array(1, 2, 3);";
        assert_eq!(transform(source), "<?php [1, 2, 3];");
    }

    #[test]
    fn test_string_array() {
        let source = "<?php array('a', 'b', 'c');";
        assert_eq!(transform(source), "<?php ['a', 'b', 'c'];");
    }

    #[test]
    fn test_associative_array() {
        let source = "<?php array('key' => 'value');";
        assert_eq!(transform(source), "<?php ['key' => 'value'];");
    }

    #[test]
    fn test_mixed_array() {
        let source = "<?php array('a' => 1, 'b' => 2, 'c');";
        assert_eq!(transform(source), "<?php ['a' => 1, 'b' => 2, 'c'];");
    }

    #[test]
    fn test_array_in_assignment() {
        let source = "<?php $arr = array(1, 2, 3);";
        assert_eq!(transform(source), "<?php $arr = [1, 2, 3];");
    }

    #[test]
    fn test_array_in_return() {
        let source = "<?php return array('foo', 'bar');";
        assert_eq!(transform(source), "<?php return ['foo', 'bar'];");
    }

    // ==================== Nested Arrays ====================

    #[test]
    fn test_nested_array() {
        // Outermost array is transformed; inner ones remain (need another pass)
        let source = "<?php array(array(1, 2), array(3, 4));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php [array(1, 2), array(3, 4)];");
    }

    #[test]
    fn test_deeply_nested_array() {
        // Outermost array is transformed; inner ones remain (need multiple passes)
        let source = "<?php array(array(array(1)));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php [array(array(1))];");
    }

    #[test]
    fn test_nested_associative() {
        // Outermost array is transformed; inner one remains
        let source = "<?php array('a' => array('b' => 1));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php ['a' => array('b' => 1)];");
    }

    // ==================== Multiple Arrays ====================

    #[test]
    fn test_multiple_arrays() {
        let source = "<?php $a = array(1); $b = array(2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a = [1]; $b = [2];");
    }

    #[test]
    fn test_arrays_in_expression() {
        let source = "<?php $result = array_merge(array(1), array(2));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $result = array_merge([1], [2]);");
    }

    // ==================== Whitespace Preservation ====================

    #[test]
    fn test_preserve_whitespace() {
        let source = "<?php array( 1, 2, 3 );";
        assert_eq!(transform(source), "<?php [ 1, 2, 3 ];");
    }

    #[test]
    fn test_preserve_multiline() {
        let source = r#"<?php array(
    'a' => 1,
    'b' => 2,
);"#;
        let expected = r#"<?php [
    'a' => 1,
    'b' => 2,
];"#;
        assert_eq!(transform(source), expected);
    }

    #[test]
    fn test_preserve_comments() {
        let source = "<?php array(/* comment */ 1, 2);";
        assert_eq!(transform(source), "<?php [/* comment */ 1, 2];");
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_array_in_function() {
        let source = r#"<?php
function getItems() {
    return array('item1', 'item2');
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_array_in_class() {
        let source = r#"<?php
class Foo {
    private $items = array();

    public function getDefaults() {
        return array('a' => 1);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_array_in_foreach() {
        let source = "<?php foreach (array(1, 2, 3) as $item) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_array_in_if_condition() {
        let source = "<?php if (in_array($x, array('a', 'b'))) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_array() {
        let source = "<?php ARRAY(1, 2, 3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php [1, 2, 3];");
    }

    #[test]
    fn test_mixed_case_array() {
        let source = "<?php Array(1, 2, 3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_short_syntax() {
        // Already using short syntax
        let source = "<?php [1, 2, 3];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_empty_short_syntax() {
        let source = "<?php [];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_array_with_trailing_comma() {
        let source = "<?php array(1, 2, 3,);";
        assert_eq!(transform(source), "<?php [1, 2, 3,];");
    }

    #[test]
    fn test_array_with_spread() {
        let source = "<?php array(...$items);";
        assert_eq!(transform(source), "<?php [...$items];");
    }

    #[test]
    fn test_array_with_expressions() {
        let source = "<?php array($a + $b, func(), $obj->method());";
        assert_eq!(transform(source), "<?php [$a + $b, func(), $obj->method()];");
    }

    #[test]
    fn test_deeply_nested_context() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($condition) {
            foreach ($items as $item) {
                $data = array('key' => $item);
            }
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_syntax() {
        // Mix of array() and [] - only transform the legacy ones
        let source = "<?php $a = array(1); $b = [2]; $c = array(3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a = [1]; $b = [2]; $c = [3];");
    }
}
