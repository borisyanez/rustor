//! Rule: Convert $x = $x ?? $default to $x ??= $default
//!
//! The null coalescing assignment operator (??=) was introduced in PHP 7.4
//! and provides a more concise syntax for this common pattern.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for null coalescing assignment patterns
pub fn check_assign_coalesce<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = AssignCoalesceVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct AssignCoalesceVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for AssignCoalesceVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_assign_coalesce(expr, self.source) {
            self.edits.push(edit);
            return false; // Don't traverse children
        }
        true // Continue traversal
    }
}

/// Try to transform an assignment with coalesce, returning the Edit if successful
fn try_transform_assign_coalesce(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    // Match assignment expression: $x = something
    let assign = match expr {
        Expression::Assignment(a) => a,
        _ => return None,
    };

    // Only handle simple assignment (=), not compound assignments (+=, etc.)
    if !matches!(assign.operator, AssignmentOperator::Assign(_)) {
        return None;
    }

    // Check if RHS is a coalesce expression: $x ?? $default
    let binary = match &*assign.rhs {
        Expression::Binary(b) => b,
        _ => return None,
    };

    // Check for null coalesce operator (??)
    if !matches!(binary.operator, BinaryOperator::NullCoalesce(_)) {
        return None;
    }

    // Get the code for LHS of assignment and LHS of coalesce
    let assign_lhs_span = assign.lhs.span();
    let assign_lhs_code =
        &source[assign_lhs_span.start.offset as usize..assign_lhs_span.end.offset as usize];

    let coalesce_lhs_span = binary.lhs.span();
    let coalesce_lhs_code =
        &source[coalesce_lhs_span.start.offset as usize..coalesce_lhs_span.end.offset as usize];

    // The LHS of assignment must match the LHS of coalesce
    // e.g., $x = $x ?? $default (match) vs $x = $y ?? $default (no match)
    if assign_lhs_code != coalesce_lhs_code {
        return None;
    }

    // Get the default value (RHS of coalesce)
    let default_span = binary.rhs.span();
    let default_code = &source[default_span.start.offset as usize..default_span.end.offset as usize];

    // Build replacement: $x ??= $default
    let replacement = format!("{} ??= {}", assign_lhs_code, default_code);

    Some(Edit::new(
        expr.span(),
        replacement,
        "Replace $x = $x ?? $default with $x ??= $default",
    ))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct AssignCoalesceRule;

impl Rule for AssignCoalesceRule {
    fn name(&self) -> &'static str {
        "assign_coalesce"
    }

    fn description(&self) -> &'static str {
        "Convert $x = $x ?? $default to $x ??= $default"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_assign_coalesce(program, source)
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php74)
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
        check_assign_coalesce(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_assign_coalesce() {
        let source = "<?php $x = $x ?? 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x ??= 'default';");
    }

    #[test]
    fn test_assign_coalesce_with_array_access() {
        let source = "<?php $arr['key'] = $arr['key'] ?? [];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $arr['key'] ??= [];");
    }

    #[test]
    fn test_assign_coalesce_with_property() {
        let source = "<?php $this->value = $this->value ?? null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $this->value ??= null;");
    }

    #[test]
    fn test_assign_coalesce_with_static_property() {
        let source = "<?php self::$cache = self::$cache ?? [];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php self::$cache ??= [];");
    }

    #[test]
    fn test_assign_coalesce_with_function_call_default() {
        let source = "<?php $config = $config ?? getDefaultConfig();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $config ??= getDefaultConfig();");
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_assign_coalesce() {
        let source = "<?php $a = $a ?? 1; $b = $b ?? 2;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a ??= 1; $b ??= 2;");
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_different_variables() {
        // $x = $y ?? $default should NOT be transformed
        let source = "<?php $x = $y ?? 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_compound_assignment() {
        // Already using compound assignment
        let source = "<?php $x ??= 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_coalesce_binary() {
        let source = "<?php $x = $x + 1;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_ternary() {
        // isset pattern should not match
        let source = "<?php $x = isset($x) ? $x : 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_in_if_statement() {
        let source = r#"<?php
if ($condition) {
    $options = $options ?? [];
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_function() {
        let source = r#"<?php
function init() {
    $config = $config ?? [];
    return $config;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_class_method() {
        let source = r#"<?php
class Foo {
    public function init() {
        $this->items = $this->items ?? [];
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_deeply_nested() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($x) {
            foreach ($items as $item) {
                $item->data = $item->data ?? [];
            }
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
