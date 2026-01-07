//! Rule: Convert list() to short array syntax
//!
//! The list() construct can be replaced with short array syntax [] for
//! destructuring assignments. This was introduced in PHP 7.1.
//!
//! Example: `list($a, $b) = $arr` → `[$a, $b] = $arr`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for list() that can be replaced with []
pub fn check_list_short_syntax<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ListShortSyntaxVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ListShortSyntaxVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ListShortSyntaxVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_list(expr, self.source) {
            self.edits.push(edit);
            return false; // Don't traverse children to avoid overlapping edits
        }
        true // Continue traversal
    }
}

/// Try to transform a list() expression to short array syntax
fn try_transform_list(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    if let Expression::List(list) = expr {
        let list_span = list.span();
        let list_source = &source[list_span.start.offset as usize..list_span.end.offset as usize];

        // Check if it starts with "list" (case-insensitive)
        let lower = list_source.to_lowercase();
        if !lower.starts_with("list") {
            return None; // Already using short syntax
        }

        // Find the opening parenthesis after "list"
        let paren_start = list_source.find('(')?;
        // Get the content between parentheses (excluding the parens)
        let inner_content = &list_source[paren_start + 1..list_source.len() - 1];

        // Build replacement with short syntax
        let replacement = format!("[{}]", inner_content);

        return Some(Edit::new(
            list_span,
            replacement,
            "Replace list() with [] (short list syntax)",
        ));
    }
    None
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct ListShortSyntaxRule;

impl Rule for ListShortSyntaxRule {
    fn name(&self) -> &'static str {
        "list_short_syntax"
    }

    fn description(&self) -> &'static str {
        "Convert list($a, $b) to [$a, $b]"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_list_short_syntax(program, source)
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php71)
    }

    fn category(&self) -> Category {
        Category::Modernization
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
        check_list_short_syntax(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_list() {
        let source = "<?php list($a, $b) = $arr;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php [$a, $b] = $arr;");
    }

    #[test]
    fn test_list_three_vars() {
        let source = "<?php list($a, $b, $c) = $arr;";
        assert_eq!(transform(source), "<?php [$a, $b, $c] = $arr;");
    }

    #[test]
    fn test_list_single_var() {
        let source = "<?php list($a) = $arr;";
        assert_eq!(transform(source), "<?php [$a] = $arr;");
    }

    #[test]
    fn test_list_with_keys() {
        let source = r#"<?php list("a" => $a, "b" => $b) = $arr;"#;
        assert_eq!(transform(source), r#"<?php ["a" => $a, "b" => $b] = $arr;"#);
    }

    #[test]
    fn test_list_with_skip() {
        // Skip elements with empty slots
        let source = "<?php list($a, , $c) = $arr;";
        assert_eq!(transform(source), "<?php [$a, , $c] = $arr;");
    }

    #[test]
    fn test_list_in_foreach_body() {
        // list() in foreach body is supported
        let source = "<?php foreach ($items as $item) { list($a, $b) = $item; }";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_list_in_foreach_header() {
        // list() in foreach header is not currently traversed by visitor
        let source = "<?php foreach ($items as list($key, $value)) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0); // Not supported yet
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_list_in_function() {
        let source = r#"<?php
function test($arr) {
    list($a, $b) = $arr;
    return $a + $b;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_list_in_class_method() {
        let source = r#"<?php
class Foo {
    public function bar($arr) {
        list($x, $y) = $arr;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_list_in_if() {
        let source = "<?php if ($condition) { list($a, $b) = $arr; }";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_list_in_while() {
        let source = "<?php while ($row = getRow()) { list($id, $name) = $row; }";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_list() {
        let source = "<?php list($a, $b) = $arr1; list($c, $d) = $arr2;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php [$a, $b] = $arr1; [$c, $d] = $arr2;");
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_list() {
        let source = "<?php LIST($a, $b) = $arr;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php [$a, $b] = $arr;");
    }

    #[test]
    fn test_mixed_case_list() {
        let source = "<?php List($a, $b) = $arr;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_short_syntax() {
        // Already using short syntax
        let source = "<?php [$a, $b] = $arr;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_short_syntax_with_keys() {
        let source = r#"<?php ["a" => $a, "b" => $b] = $arr;"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_nested_list() {
        // Only outermost list is transformed
        let source = "<?php list($a, list($b, $c)) = $arr;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        // The inner list() is kept as-is in first pass
        assert_eq!(transform(source), "<?php [$a, list($b, $c)] = $arr;");
    }

    #[test]
    fn test_list_with_array_access() {
        let source = "<?php list($a, $b) = $arr[0];";
        assert_eq!(transform(source), "<?php [$a, $b] = $arr[0];");
    }

    #[test]
    fn test_list_with_function_call() {
        let source = "<?php list($a, $b) = getData();";
        assert_eq!(transform(source), "<?php [$a, $b] = getData();");
    }

    #[test]
    fn test_list_with_method_call() {
        let source = "<?php list($a, $b) = $obj->getData();";
        assert_eq!(transform(source), "<?php [$a, $b] = $obj->getData();");
    }

    #[test]
    fn test_deeply_nested() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($condition) {
            foreach ($items as $item) {
                list($a, $b) = $item;
            }
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_list_preserves_whitespace() {
        let source = "<?php list( $a , $b ) = $arr;";
        assert_eq!(transform(source), "<?php [ $a , $b ] = $arr;");
    }

    #[test]
    fn test_list_with_trailing_comma() {
        let source = "<?php list($a, $b,) = $arr;";
        assert_eq!(transform(source), "<?php [$a, $b,] = $arr;");
    }
}
