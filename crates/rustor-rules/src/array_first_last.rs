//! Rule: Convert array access patterns to array_first()/array_last() (PHP 8.5+)
//!
//! This rule converts patterns that access first/last array elements to the new
//! PHP 8.5 functions.
//!
//! Example:
//! ```php
//! // Before
//! $first = $array[array_key_first($array)];
//! $last = $array[array_key_last($array)];
//! $first = array_values($array)[0];
//!
//! // After
//! $first = array_first($array);
//! $last = array_last($array);
//! $first = array_first($array);
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for array first/last access patterns
pub fn check_array_first_last<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ArrayFirstLastVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ArrayFirstLastVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ArrayFirstLastVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::ArrayAccess(array_access) = expr {
            // Pattern 1 & 2: $array[array_key_first($array)] or $array[array_key_last($array)]
            if let Some(edit) = self.check_array_key_pattern(array_access) {
                self.edits.push(edit);
                return false; // Don't recurse into this expression
            }

            // Pattern 3: array_values($array)[0]
            if let Some(edit) = self.check_array_values_pattern(array_access) {
                self.edits.push(edit);
                return false;
            }
        }
        true // Continue traversal
    }
}

impl<'s> ArrayFirstLastVisitor<'s> {
    /// Check for $array[array_key_first($array)] or $array[array_key_last($array)]
    fn check_array_key_pattern(&self, array_access: &ArrayAccess<'_>) -> Option<Edit> {
        // The index must be a function call
        let Expression::Call(Call::Function(func)) = array_access.index else {
            return None;
        };

        // Get the function name
        let func_name = self.get_func_name(func)?;

        let is_first = func_name.eq_ignore_ascii_case("array_key_first");
        let is_last = func_name.eq_ignore_ascii_case("array_key_last");

        if !is_first && !is_last {
            return None;
        }

        // Must have exactly 1 argument
        let args: Vec<_> = func.argument_list.arguments.iter().collect();
        if args.len() != 1 {
            return None;
        }

        // Skip unpacked arguments
        if args[0].is_unpacked() {
            return None;
        }

        // Get the argument source
        let arg_span = args[0].span();
        let arg_source = &self.source[arg_span.start.offset as usize..arg_span.end.offset as usize];

        // Get the array source
        let array_span = array_access.array.span();
        let array_source =
            &self.source[array_span.start.offset as usize..array_span.end.offset as usize];

        // The array and argument must be the same (by source comparison)
        if !self.sources_equal(array_source, arg_source) {
            return None;
        }

        // Create replacement
        let span = array_access.span();
        let new_func = if is_first { "array_first" } else { "array_last" };
        let replacement = format!("{}({})", new_func, array_source);

        let message = if is_first {
            "Convert $arr[array_key_first($arr)] to array_first($arr) (PHP 8.5+)"
        } else {
            "Convert $arr[array_key_last($arr)] to array_last($arr) (PHP 8.5+)"
        };

        Some(Edit::new(span, replacement, message))
    }

    /// Check for array_values($array)[0] pattern
    fn check_array_values_pattern(&self, array_access: &ArrayAccess<'_>) -> Option<Edit> {
        // The array being accessed must be array_values($arr)
        let Expression::Call(Call::Function(func)) = array_access.array else {
            return None;
        };

        let func_name = self.get_func_name(func)?;

        if !func_name.eq_ignore_ascii_case("array_values") {
            return None;
        }

        // Must have exactly 1 argument
        let args: Vec<_> = func.argument_list.arguments.iter().collect();
        if args.len() != 1 {
            return None;
        }

        // Skip unpacked arguments
        if args[0].is_unpacked() {
            return None;
        }

        // The index must be 0 (for array_first)
        if !self.is_zero_literal(array_access.index) {
            return None;
        }

        // Get the inner argument source
        let arg_span = args[0].span();
        let arg_source = &self.source[arg_span.start.offset as usize..arg_span.end.offset as usize];

        // Create replacement
        let span = array_access.span();
        let replacement = format!("array_first({})", arg_source);

        Some(Edit::new(
            span,
            replacement,
            "Convert array_values($arr)[0] to array_first($arr) (PHP 8.5+)",
        ))
    }

    /// Get function name from a function call
    fn get_func_name(&self, func: &FunctionCall<'_>) -> Option<&str> {
        if let Expression::Identifier(ident) = &func.function {
            let span = ident.span();
            return Some(&self.source[span.start.offset as usize..span.end.offset as usize]);
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

    /// Compare two source strings, ignoring whitespace
    fn sources_equal(&self, a: &str, b: &str) -> bool {
        let a_normalized: String = a.chars().filter(|c| !c.is_whitespace()).collect();
        let b_normalized: String = b.chars().filter(|c| !c.is_whitespace()).collect();
        a_normalized == b_normalized
    }
}

pub struct ArrayFirstLastRule;

impl Rule for ArrayFirstLastRule {
    fn name(&self) -> &'static str {
        "array_first_last"
    }

    fn description(&self) -> &'static str {
        "Convert array access patterns to array_first()/array_last() (PHP 8.5+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_array_first_last(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php85)
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
        check_array_first_last(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== array_key_first Pattern Tests ====================

    #[test]
    fn test_array_key_first_pattern() {
        let source = r#"<?php $first = $array[array_key_first($array)];"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $first = array_first($array);");
    }

    #[test]
    fn test_array_key_first_uppercase() {
        let source = r#"<?php $first = $array[ARRAY_KEY_FIRST($array)];"#;
        assert_eq!(transform(source), "<?php $first = array_first($array);");
    }

    #[test]
    fn test_array_key_first_property() {
        let source = r#"<?php $first = $this->items[array_key_first($this->items)];"#;
        assert_eq!(
            transform(source),
            "<?php $first = array_first($this->items);"
        );
    }

    // ==================== array_key_last Pattern Tests ====================

    #[test]
    fn test_array_key_last_pattern() {
        let source = r#"<?php $last = $array[array_key_last($array)];"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $last = array_last($array);");
    }

    #[test]
    fn test_array_key_last_uppercase() {
        let source = r#"<?php $last = $array[ARRAY_KEY_LAST($array)];"#;
        assert_eq!(transform(source), "<?php $last = array_last($array);");
    }

    #[test]
    fn test_array_key_last_property() {
        let source = r#"<?php $last = $this->data[array_key_last($this->data)];"#;
        assert_eq!(transform(source), "<?php $last = array_last($this->data);");
    }

    // ==================== array_values Pattern Tests ====================

    #[test]
    fn test_array_values_zero() {
        let source = r#"<?php $first = array_values($array)[0];"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $first = array_first($array);");
    }

    #[test]
    fn test_array_values_uppercase() {
        let source = r#"<?php $first = ARRAY_VALUES($array)[0];"#;
        assert_eq!(transform(source), "<?php $first = array_first($array);");
    }

    #[test]
    fn test_array_values_property() {
        let source = r#"<?php $first = array_values($this->items)[0];"#;
        assert_eq!(transform(source), "<?php $first = array_first($this->items);");
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_mismatched_arrays() {
        // Different arrays in outer and inner expressions
        let source = r#"<?php $first = $array[array_key_first($other)];"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_array_values_non_zero() {
        // Non-zero index
        let source = r#"<?php $second = array_values($array)[1];"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_array_values_variable_index() {
        // Variable index
        let source = r#"<?php $item = array_values($array)[$i];"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function_in_index() {
        // Different function in index
        let source = r#"<?php $item = $array[count($array)];"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_array_key_first_with_extra_args() {
        // Extra arguments (shouldn't happen, but test anyway)
        let source = r#"<?php $first = $array[array_key_first($array, $extra)];"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Multiple Patterns ====================

    #[test]
    fn test_multiple_patterns() {
        let source = r#"<?php
$first = $arr[array_key_first($arr)];
$last = $data[array_key_last($data)];
$val = array_values($items)[0];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 3);
        let result = transform(source);
        assert!(result.contains("array_first($arr)"));
        assert!(result.contains("array_last($data)"));
        assert!(result.contains("array_first($items)"));
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_echo() {
        let source = r#"<?php echo $array[array_key_first($array)];"#;
        assert_eq!(transform(source), "<?php echo array_first($array);");
    }

    #[test]
    fn test_in_return() {
        let source = r#"<?php return $array[array_key_last($array)];"#;
        assert_eq!(transform(source), "<?php return array_last($array);");
    }

    #[test]
    fn test_in_function_arg() {
        let source = r#"<?php process($array[array_key_first($array)]);"#;
        assert_eq!(transform(source), "<?php process(array_first($array));");
    }
}
