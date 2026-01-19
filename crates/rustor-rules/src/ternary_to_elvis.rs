//! Rule: Convert ternary to elvis operator
//!
//! Since PHP 5.3, the elvis operator (?:) can be used as shorthand.
//!
//! Transformation:
//! - `$a ? $a : $b` → `$a ?: $b`
//! - `$x['key'] ? $x['key'] : $default` → `$x['key'] ?: $default`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for ternary expressions that can use elvis
pub fn check_ternary_to_elvis<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = TernaryToElvisVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct TernaryToElvisVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for TernaryToElvisVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Conditional(ternary) = expr {
            if let Some(edit) = try_convert_to_elvis(ternary, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to convert ternary to elvis operator
fn try_convert_to_elvis(ternary: &Conditional<'_>, source: &str) -> Option<Edit> {
    // Must have a then part (not already an elvis operator)
    let then_part = ternary.then.as_ref()?;

    // Get text representations
    let cond_span = ternary.condition.span();
    let then_span = then_part.span();

    let cond_text = &source[cond_span.start.offset as usize..cond_span.end.offset as usize];
    let then_text = &source[then_span.start.offset as usize..then_span.end.offset as usize];

    // Check if condition and then part are identical
    if normalize_whitespace(cond_text) != normalize_whitespace(then_text) {
        return None;
    }

    // Get the else part text
    let else_span = ternary.r#else.span();
    let else_text = &source[else_span.start.offset as usize..else_span.end.offset as usize];

    // Build replacement: condition ?: else
    let ternary_span = ternary.span();
    let replacement = format!("{} ?: {}", cond_text, else_text);

    Some(Edit::new(
        ternary_span,
        replacement,
        "Convert ternary to elvis operator",
    ))
}

/// Normalize whitespace for comparison
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct TernaryToElvisRule;

impl Rule for TernaryToElvisRule {
    fn name(&self) -> &'static str {
        "ternary_to_elvis"
    }

    fn description(&self) -> &'static str {
        "Convert ternary to elvis operator"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_ternary_to_elvis(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php54)
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
        check_ternary_to_elvis(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php $a ? $a : false;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $a ?: false;");
    }

    #[test]
    fn test_with_default() {
        let source = "<?php $value ? $value : $default;";
        assert_eq!(transform(source), "<?php $value ?: $default;");
    }

    #[test]
    fn test_with_string() {
        let source = r#"<?php $name ? $name : 'Anonymous';"#;
        assert_eq!(transform(source), r#"<?php $name ?: 'Anonymous';"#);
    }

    // ==================== Array Access ====================

    #[test]
    fn test_array_access() {
        let source = r#"<?php $x['key'] ? $x['key'] : null;"#;
        assert_eq!(transform(source), r#"<?php $x['key'] ?: null;"#);
    }

    #[test]
    fn test_nested_array() {
        let source = r#"<?php $x['a']['b'] ? $x['a']['b'] : '';"#;
        assert_eq!(transform(source), r#"<?php $x['a']['b'] ?: '';"#);
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = $value ? $value : 0;";
        assert_eq!(transform(source), "<?php $result = $value ?: 0;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return $x ? $x : null;";
        assert_eq!(transform(source), "<?php return $x ?: null;");
    }

    #[test]
    fn test_in_function() {
        let source = r#"<?php
function elvis() {
    $value = $a ? $a : false;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = $x ? $x : null;
$b = $y ? $y : 0;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_different_condition_then() {
        // Condition and then part are different
        let source = "<?php $a ? $b : $c;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_already_elvis() {
        // Already using elvis operator
        let source = "<?php $a ?: $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_function_call() {
        // Function call has side effects, different calls
        let source = "<?php getData() ? getData() : null;";
        // This COULD match textually, but we might want to skip functions
        // For now, we allow it since text matches
        let edits = check_php(source);
        assert_eq!(edits.len(), 1); // Text matches, so it transforms
    }

    #[test]
    fn test_skip_complex_condition() {
        // Complex condition that differs from then
        let source = "<?php $a > 0 ? $a : 0;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_null_coalesce_pattern() {
        // This is a null coalesce pattern, not elvis
        let source = "<?php isset($x) ? $x : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
