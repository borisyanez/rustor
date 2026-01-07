//! Rule: Convert array key access patterns to array_key_first()/array_key_last() (PHP 7.3+)
//!
//! Example:
//! ```php
//! // Before
//! $first = array_keys($arr)[0];
//! $first = reset(array_keys($arr));
//! $last = end(array_keys($arr));
//!
//! // After
//! $first = array_key_first($arr);
//! $first = array_key_first($arr);
//! $last = array_key_last($arr);
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for array key access patterns
pub fn check_array_key_first_last<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ArrayKeyFirstLastVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ArrayKeyFirstLastVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ArrayKeyFirstLastVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        match expr {
            // Check for array_keys($arr)[0] pattern
            Expression::ArrayAccess(array_access) => {
                self.check_array_access(array_access);
            }
            // Check for reset(array_keys($arr)) or end(array_keys($arr)) patterns
            Expression::Call(Call::Function(func)) => {
                self.check_reset_end_call(func);
            }
            _ => {}
        }
        true // Continue traversal
    }
}

impl<'s> ArrayKeyFirstLastVisitor<'s> {
    /// Check for array_keys($arr)[0] pattern
    fn check_array_access(&mut self, array_access: &ArrayAccess<'_>) {
        // The array being accessed must be array_keys($arr)
        let inner_arg = match self.extract_array_keys_arg(array_access.array) {
            Some(arg) => arg,
            None => return,
        };

        // The index must be 0 (for array_key_first)
        if !self.is_zero_literal(array_access.index) {
            return;
        }

        // Create replacement
        let span = array_access.span();
        let replacement = format!("array_key_first({})", inner_arg);

        self.edits.push(Edit::new(
            span,
            replacement,
            "Convert array_keys()[0] to array_key_first() (PHP 7.3+)",
        ));
    }

    /// Check for reset(array_keys($arr)) or end(array_keys($arr)) patterns
    fn check_reset_end_call(&mut self, func: &FunctionCall<'_>) {
        // Get the function name
        let name = match &func.function {
            Expression::Identifier(ident) => {
                let span = ident.span();
                &self.source[span.start.offset as usize..span.end.offset as usize]
            }
            _ => return,
        };

        let is_reset = name.eq_ignore_ascii_case("reset");
        let is_end = name.eq_ignore_ascii_case("end");

        if !is_reset && !is_end {
            return;
        }

        // Must have exactly 1 argument
        let args: Vec<_> = func.argument_list.arguments.iter().collect();
        if args.len() != 1 {
            return;
        }

        // Skip unpacked arguments
        if args[0].is_unpacked() {
            return;
        }

        // The argument must be array_keys($arr)
        let inner_arg = match self.extract_array_keys_arg(args[0].value()) {
            Some(arg) => arg,
            None => return,
        };

        // Create replacement
        let span = func.span();
        let replacement = if is_reset {
            format!("array_key_first({})", inner_arg)
        } else {
            format!("array_key_last({})", inner_arg)
        };

        let message = if is_reset {
            "Convert reset(array_keys()) to array_key_first() (PHP 7.3+)"
        } else {
            "Convert end(array_keys()) to array_key_last() (PHP 7.3+)"
        };

        self.edits.push(Edit::new(span, replacement, message));
    }

    /// Extract the argument from an array_keys() call
    fn extract_array_keys_arg(&self, expr: &Expression<'_>) -> Option<String> {
        if let Expression::Call(Call::Function(func)) = expr {
            // Check if it's array_keys
            let name = match &func.function {
                Expression::Identifier(ident) => {
                    let span = ident.span();
                    &self.source[span.start.offset as usize..span.end.offset as usize]
                }
                _ => return None,
            };

            if !name.eq_ignore_ascii_case("array_keys") {
                return None;
            }

            // Must have exactly 1 argument (no filter key or strict mode)
            let args: Vec<_> = func.argument_list.arguments.iter().collect();
            if args.len() != 1 {
                return None;
            }

            // Skip unpacked arguments
            if args[0].is_unpacked() {
                return None;
            }

            // Return the argument source code
            let arg_span = args[0].span();
            return Some(
                self.source[arg_span.start.offset as usize..arg_span.end.offset as usize]
                    .to_string(),
            );
        }
        None
    }

    /// Check if expression is the integer literal 0
    fn is_zero_literal(&self, expr: &Expression<'_>) -> bool {
        if let Expression::Literal(Literal::Integer(int_lit)) = expr {
            return int_lit.value == Some(0);
        }
        false
    }
}

pub struct ArrayKeyFirstLastRule;

impl Rule for ArrayKeyFirstLastRule {
    fn name(&self) -> &'static str {
        "array_key_first_last"
    }

    fn description(&self) -> &'static str {
        "Convert array_keys()[0] to array_key_first()/array_key_last()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_array_key_first_last(program, source)
    }

    fn category(&self) -> Category {
        Category::Performance
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php73)
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
        check_array_key_first_last(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== array_keys()[0] Tests ====================

    #[test]
    fn test_array_keys_index_zero() {
        let source = r#"<?php
$first = array_keys($arr)[0];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("array_key_first($arr)"));
    }

    #[test]
    fn test_array_keys_with_variable() {
        let source = r#"<?php
$first = array_keys($data)[0];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("array_key_first($data)"));
    }

    #[test]
    fn test_array_keys_with_property() {
        let source = r#"<?php
$first = array_keys($this->items)[0];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("array_key_first($this->items)"));
    }

    // ==================== reset(array_keys()) Tests ====================

    #[test]
    fn test_reset_array_keys() {
        let source = r#"<?php
$first = reset(array_keys($arr));
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("array_key_first($arr)"));
    }

    #[test]
    fn test_reset_array_keys_uppercase() {
        let source = r#"<?php
$first = RESET(ARRAY_KEYS($arr));
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("array_key_first($arr)"));
    }

    // ==================== end(array_keys()) Tests ====================

    #[test]
    fn test_end_array_keys() {
        let source = r#"<?php
$last = end(array_keys($arr));
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("array_key_last($arr)"));
    }

    #[test]
    fn test_end_array_keys_uppercase() {
        let source = r#"<?php
$last = END(ARRAY_KEYS($arr));
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("array_key_last($arr)"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_array_keys_non_zero_index() {
        // Index is not 0
        let source = r#"<?php
$second = array_keys($arr)[1];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_array_keys_with_filter() {
        // array_keys with filter value shouldn't be converted
        let source = r#"<?php
$first = array_keys($arr, 'value')[0];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_reset_non_array_keys() {
        // reset() on non-array_keys call
        let source = r#"<?php
$first = reset($arr);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_end_non_array_keys() {
        // end() on non-array_keys call
        let source = r#"<?php
$last = end($arr);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_array_keys_variable_index() {
        // Variable index
        let source = r#"<?php
$item = array_keys($arr)[$index];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_patterns() {
        let source = r#"<?php
$first = array_keys($arr)[0];
$last = end(array_keys($data));
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        let result = transform(source);
        assert!(result.contains("array_key_first($arr)"));
        assert!(result.contains("array_key_last($data)"));
    }
}
