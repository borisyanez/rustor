//! Rule: Convert substr() comparisons to str_starts_with()/str_ends_with() (PHP 8.0+)
//!
//! Example:
//! ```php
//! // Before
//! if (substr($path, 0, 5) === '/api/') { }
//! if (substr($file, -4) === '.php') { }
//! if ('/api/' === substr($path, 0, 5)) { }
//!
//! // After
//! if (str_starts_with($path, '/api/')) { }
//! if (str_ends_with($file, '.php')) { }
//! if (str_starts_with($path, '/api/')) { }
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for substr() comparison patterns
pub fn check_string_starts_ends<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = StringStartsEndsVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StringStartsEndsVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for StringStartsEndsVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            self.check_binary_expression(binary);
        }
        true // Continue traversal
    }
}

/// Information extracted from a substr() call
struct SubstrInfo {
    haystack: String,
    offset: i64,
    length: Option<i64>,
}

impl<'s> StringStartsEndsVisitor<'s> {
    fn check_binary_expression(&mut self, binary: &Binary<'_>) {
        // Only handle === and !== comparisons
        let is_negated = match &binary.operator {
            BinaryOperator::Identical(_) => false,
            BinaryOperator::NotIdentical(_) => true,
            _ => return,
        };

        let (lhs, rhs) = (binary.lhs, binary.rhs);

        // Try substr($x, 0, n) === 'string'
        if let (Some(substr_info), Some(needle)) = (self.extract_substr_call(lhs), self.extract_string_literal(rhs)) {
            if let Some(edit) = self.create_edit(binary, &substr_info, &needle, is_negated) {
                self.edits.push(edit);
                return;
            }
        }

        // Try 'string' === substr($x, 0, n)
        if let (Some(needle), Some(substr_info)) = (self.extract_string_literal(lhs), self.extract_substr_call(rhs)) {
            if let Some(edit) = self.create_edit(binary, &substr_info, &needle, is_negated) {
                self.edits.push(edit);
            }
        }
    }

    /// Extract information from a substr() call
    fn extract_substr_call(&self, expr: &Expression<'_>) -> Option<SubstrInfo> {
        if let Expression::Call(Call::Function(func)) = expr {
            // Check if it's a substr call
            let name: &str = match func.function {
                Expression::Identifier(ident) => {
                    let span = ident.span();
                    &self.source[span.start.offset as usize..span.end.offset as usize]
                }
                _ => return None,
            };

            if !name.eq_ignore_ascii_case("substr") {
                return None;
            }

            // Get 2 or 3 arguments
            let args: Vec<_> = func.argument_list.arguments.iter().collect();
            if args.len() < 2 || args.len() > 3 {
                return None;
            }

            // Skip unpacked arguments
            if args.iter().any(|a| a.is_unpacked()) {
                return None;
            }

            // Extract haystack
            let haystack_span = args[0].span();
            let haystack = self.source[haystack_span.start.offset as usize..haystack_span.end.offset as usize].to_string();

            // Extract offset (must be a literal integer)
            let offset = self.extract_integer_literal(args[1].value())?;

            // Extract length if present
            let length = if args.len() == 3 {
                Some(self.extract_integer_literal(args[2].value())?)
            } else {
                None
            };

            return Some(SubstrInfo { haystack, offset, length });
        }
        None
    }

    /// Extract a string literal value
    fn extract_string_literal(&self, expr: &Expression<'_>) -> Option<String> {
        if let Expression::Literal(Literal::String(string_lit)) = expr {
            let span = string_lit.span();
            // Return the full string including quotes
            return Some(self.source[span.start.offset as usize..span.end.offset as usize].to_string());
        }
        None
    }

    /// Extract an integer literal value
    fn extract_integer_literal(&self, expr: &Expression<'_>) -> Option<i64> {
        match expr {
            Expression::Literal(Literal::Integer(int_lit)) => {
                int_lit.value.map(|v| v as i64)
            }
            Expression::UnaryPrefix(unary) => {
                // Handle negative numbers like -4
                if let UnaryPrefixOperator::Negation(_) = &unary.operator {
                    if let Expression::Literal(Literal::Integer(int_lit)) = unary.operand {
                        return int_lit.value.map(|v| -(v as i64));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Create an edit if the substr pattern matches str_starts_with or str_ends_with
    fn create_edit(
        &self,
        binary: &Binary<'_>,
        substr_info: &SubstrInfo,
        needle: &str,
        is_negated: bool,
    ) -> Option<Edit> {
        // Get the actual string content (without quotes) to check length
        let needle_content = &needle[1..needle.len()-1];
        let needle_len = needle_content.len() as i64;

        // Check for str_starts_with pattern: substr($x, 0, n) === 'str' where n == len('str')
        if substr_info.offset == 0 {
            if let Some(length) = substr_info.length {
                if length == needle_len {
                    let span = binary.span();
                    let func = if is_negated {
                        format!("!str_starts_with({}, {})", substr_info.haystack, needle)
                    } else {
                        format!("str_starts_with({}, {})", substr_info.haystack, needle)
                    };
                    return Some(Edit::new(
                        span,
                        func,
                        "Convert substr() to str_starts_with() (PHP 8.0+)",
                    ));
                }
            }
        }

        // Check for str_ends_with pattern: substr($x, -n) === 'str' where n == len('str')
        if substr_info.offset < 0 && substr_info.length.is_none() {
            if substr_info.offset.abs() == needle_len {
                let span = binary.span();
                let func = if is_negated {
                    format!("!str_ends_with({}, {})", substr_info.haystack, needle)
                } else {
                    format!("str_ends_with({}, {})", substr_info.haystack, needle)
                };
                return Some(Edit::new(
                    span,
                    func,
                    "Convert substr() to str_ends_with() (PHP 8.0+)",
                ));
            }
        }

        None
    }
}

pub struct StringStartsEndsRule;

impl Rule for StringStartsEndsRule {
    fn name(&self) -> &'static str {
        "string_starts_ends"
    }

    fn description(&self) -> &'static str {
        "Convert substr() comparisons to str_starts_with()/str_ends_with()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_string_starts_ends(program, source)
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
        check_string_starts_ends(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== str_starts_with Tests ====================

    #[test]
    fn test_starts_with_basic() {
        let source = r#"<?php
if (substr($path, 0, 5) === '/api/') {
    echo 'api route';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_starts_with($path, '/api/')"));
    }

    #[test]
    fn test_starts_with_single_char() {
        let source = r#"<?php
if (substr($str, 0, 1) === '/') {
    echo 'starts with slash';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_starts_with($str, '/')"));
    }

    #[test]
    fn test_starts_with_reversed() {
        let source = r#"<?php
if ('/api/' === substr($path, 0, 5)) {
    echo 'api route';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_starts_with($path, '/api/')"));
    }

    #[test]
    fn test_starts_with_negated() {
        let source = r#"<?php
if (substr($path, 0, 5) !== '/api/') {
    echo 'not api';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("!str_starts_with($path, '/api/')"));
    }

    // ==================== str_ends_with Tests ====================

    #[test]
    fn test_ends_with_basic() {
        let source = r#"<?php
if (substr($file, -4) === '.php') {
    echo 'php file';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_ends_with($file, '.php')"));
    }

    #[test]
    fn test_ends_with_single_char() {
        let source = r#"<?php
if (substr($path, -1) === '/') {
    echo 'ends with slash';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_ends_with($path, '/')"));
    }

    #[test]
    fn test_ends_with_reversed() {
        let source = r#"<?php
if ('.php' === substr($file, -4)) {
    echo 'php file';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_ends_with($file, '.php')"));
    }

    #[test]
    fn test_ends_with_negated() {
        let source = r#"<?php
if (substr($file, -4) !== '.php') {
    echo 'not php';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("!str_ends_with($file, '.php')"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_length_mismatch() {
        // Length doesn't match string length
        let source = r#"<?php
if (substr($path, 0, 10) === '/api/') {
    echo 'mismatch';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_zero_offset_with_length() {
        // Non-zero offset with length is not starts_with
        let source = r#"<?php
if (substr($path, 1, 4) === 'api/') {
    echo 'not at start';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_negative_offset_with_length() {
        // Negative offset with length is not ends_with
        let source = r#"<?php
if (substr($file, -4, 3) === '.ph') {
    echo 'partial';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_variable_comparison() {
        // Comparison with variable, not literal
        let source = r#"<?php
if (substr($path, 0, 5) === $prefix) {
    echo 'variable';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_loose_comparison() {
        // Loose comparison shouldn't be converted
        let source = r#"<?php
if (substr($path, 0, 5) == '/api/') {
    echo 'loose';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_patterns() {
        let source = r#"<?php
if (substr($path, 0, 5) === '/api/' && substr($file, -4) === '.php') {
    echo 'api php file';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        let result = transform(source);
        assert!(result.contains("str_starts_with($path, '/api/')"));
        assert!(result.contains("str_ends_with($file, '.php')"));
    }
}
