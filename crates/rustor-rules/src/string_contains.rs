//! Rule: Convert strpos() !== false to str_contains() (PHP 8.0+)
//!
//! Example:
//! ```php
//! // Before
//! if (strpos($text, 'needle') !== false) { }
//! if (strpos($text, 'needle') === false) { }
//! if (false !== strpos($text, 'needle')) { }
//! if (false === strpos($text, 'needle')) { }
//!
//! // After
//! if (str_contains($text, 'needle')) { }
//! if (!str_contains($text, 'needle')) { }
//! if (str_contains($text, 'needle')) { }
//! if (!str_contains($text, 'needle')) { }
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for strpos() !== false patterns
pub fn check_string_contains<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = StringContainsVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StringContainsVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for StringContainsVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            self.check_binary_expression(binary);
        }
        true // Continue traversal
    }
}

impl<'s> StringContainsVisitor<'s> {
    fn check_binary_expression(&mut self, binary: &Binary<'_>) {
        // Match patterns:
        // - strpos($x, $y) !== false
        // - strpos($x, $y) === false
        // - false !== strpos($x, $y)
        // - false === strpos($x, $y)

        let is_negated = match &binary.operator {
            BinaryOperator::NotIdentical(_) => true,  // !== false means "contains"
            BinaryOperator::Identical(_) => false,    // === false means "not contains"
            _ => return,
        };

        // Try both orderings
        let (lhs, rhs) = (binary.lhs, binary.rhs);

        // Check strpos($x, $y) !== false
        if let (Some((haystack, needle)), true) = (self.extract_strpos_call(lhs), self.is_false_literal(rhs)) {
            self.create_edit(binary, &haystack, &needle, is_negated);
            return;
        }

        // Check false !== strpos($x, $y)
        if let (true, Some((haystack, needle))) = (self.is_false_literal(lhs), self.extract_strpos_call(rhs)) {
            self.create_edit(binary, &haystack, &needle, is_negated);
        }
    }

    /// Extract haystack and needle from a strpos() call
    fn extract_strpos_call(&self, expr: &Expression<'_>) -> Option<(String, String)> {
        if let Expression::Call(Call::Function(func)) = expr {
            // Check if it's a strpos call
            let name: &str = match func.function {
                Expression::Identifier(ident) => {
                    let span = ident.span();
                    &self.source[span.start.offset as usize..span.end.offset as usize]
                }
                _ => return None,
            };

            if !name.eq_ignore_ascii_case("strpos") {
                return None;
            }

            // Get exactly 2 arguments (no offset parameter)
            let args: Vec<_> = func.argument_list.arguments.iter().collect();
            if args.len() != 2 {
                return None;
            }

            // Skip unpacked arguments
            if args[0].is_unpacked() || args[1].is_unpacked() {
                return None;
            }

            // Extract argument code from source
            let haystack_span = args[0].span();
            let needle_span = args[1].span();

            let haystack = self.source[haystack_span.start.offset as usize..haystack_span.end.offset as usize].to_string();
            let needle = self.source[needle_span.start.offset as usize..needle_span.end.offset as usize].to_string();

            return Some((haystack, needle));
        }
        None
    }

    /// Check if expression is the false literal
    fn is_false_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::False(_)))
    }

    fn create_edit(
        &mut self,
        binary: &Binary<'_>,
        haystack: &str,
        needle: &str,
        is_negated: bool,
    ) {
        let span = binary.span();

        let replacement = if is_negated {
            // !== false means "contains", so no negation
            format!("str_contains({}, {})", haystack, needle)
        } else {
            // === false means "does not contain", so negate
            format!("!str_contains({}, {})", haystack, needle)
        };

        self.edits.push(Edit::new(
            span,
            replacement,
            "Convert strpos() to str_contains() (PHP 8.0+)",
        ));
    }
}

pub struct StringContainsRule;

impl Rule for StringContainsRule {
    fn name(&self) -> &'static str {
        "string_contains"
    }

    fn description(&self) -> &'static str {
        "Convert strpos() !== false to str_contains()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_string_contains(program, source)
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
        check_string_contains(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_strpos_not_identical_false() {
        let source = r#"<?php
if (strpos($text, 'needle') !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains($text, 'needle')"));
        assert!(!result.contains("!str_contains"));
    }

    #[test]
    fn test_strpos_identical_false() {
        let source = r#"<?php
if (strpos($text, 'needle') === false) {
    echo 'not found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("!str_contains($text, 'needle')"));
    }

    #[test]
    fn test_false_not_identical_strpos() {
        let source = r#"<?php
if (false !== strpos($text, 'needle')) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains($text, 'needle')"));
        assert!(!result.contains("!str_contains"));
    }

    #[test]
    fn test_false_identical_strpos() {
        let source = r#"<?php
if (false === strpos($text, 'needle')) {
    echo 'not found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("!str_contains($text, 'needle')"));
    }

    #[test]
    fn test_variable_needle() {
        let source = r#"<?php
if (strpos($haystack, $needle) !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains($haystack, $needle)"));
    }

    #[test]
    fn test_function_call_as_haystack() {
        let source = r#"<?php
if (strpos(strtolower($text), 'needle') !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains(strtolower($text), 'needle')"));
    }

    #[test]
    fn test_in_ternary() {
        let source = r#"<?php
$result = strpos($text, 'x') !== false ? 'yes' : 'no';
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains($text, 'x')"));
    }

    #[test]
    fn test_in_return() {
        let source = r#"<?php
return strpos($text, 'x') !== false;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("return str_contains($text, 'x')"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_strpos_with_offset() {
        // strpos with offset (3rd argument) shouldn't be converted
        let source = r#"<?php
if (strpos($text, 'needle', 5) !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_strpos_compared_to_number() {
        // strpos compared to a number (checking position) shouldn't be converted
        let source = r#"<?php
if (strpos($text, 'needle') === 0) {
    echo 'starts with';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_strpos_greater_than() {
        let source = r#"<?php
if (strpos($text, 'needle') > 0) {
    echo 'found after start';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_strpos_loose_comparison() {
        // Loose comparison (== or !=) shouldn't be converted
        let source = r#"<?php
if (strpos($text, 'needle') != false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_stripos() {
        // stripos is case-insensitive, str_contains is case-sensitive
        let source = r#"<?php
if (stripos($text, 'needle') !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_strpos_checks() {
        let source = r#"<?php
if (strpos($a, 'x') !== false && strpos($b, 'y') !== false) {
    echo 'both found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        let result = transform(source);
        assert!(result.contains("str_contains($a, 'x')"));
        assert!(result.contains("str_contains($b, 'y')"));
    }
}
