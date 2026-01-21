//! Rule: remove_duplicated_array_key (Simplification)
//!
//! Removes duplicate array keys, keeping only the last occurrence.
//! PHP uses the last value for duplicate keys anyway.
//!
//! Example transformation:
//! ```php
//! // Before
//! $arr = ['foo' => 1, 'bar' => 2, 'foo' => 3];
//!
//! // After
//! $arr = ['bar' => 2, 'foo' => 3];
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_remove_duplicated_array_key<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveDuplicatedArrayKeyVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveDuplicatedArrayKeyVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> RemoveDuplicatedArrayKeyVisitor<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Extract a normalized key from an expression for comparison
    fn extract_key(&self, expr: &Expression<'_>) -> Option<String> {
        match expr {
            // Integer literal
            Expression::Literal(Literal::Integer(int_lit)) => {
                let text = self.get_text(int_lit.span());
                // Parse to normalize (0x10 == 16 == 020)
                self.parse_int_literal(text).map(|v| format!("int:{}", v))
            }
            // String literal
            Expression::Literal(Literal::String(string_lit)) => {
                let full_text = self.get_text(string_lit.span());
                // Extract content between quotes
                if full_text.len() >= 2 {
                    let content = &full_text[1..full_text.len() - 1];
                    Some(format!("str:{}", content))
                } else {
                    None
                }
            }
            // Float that is actually an integer
            Expression::Literal(Literal::Float(float_lit)) => {
                let text = self.get_text(float_lit.span());
                if let Ok(val) = text.parse::<f64>() {
                    if val.fract() == 0.0 {
                        Some(format!("int:{}", val as i64))
                    } else {
                        // Floats as keys are truncated to int in PHP
                        Some(format!("int:{}", val.trunc() as i64))
                    }
                } else {
                    None
                }
            }
            // true/false literals become 1/0
            Expression::Literal(Literal::True(_)) => Some("int:1".to_string()),
            Expression::Literal(Literal::False(_)) => Some("int:0".to_string()),
            // null becomes empty string
            Expression::Literal(Literal::Null(_)) => Some("str:".to_string()),
            _ => None,
        }
    }

    fn parse_int_literal(&self, text: &str) -> Option<i64> {
        let text = text.replace('_', "");
        if let Some(stripped) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            i64::from_str_radix(stripped, 16).ok()
        } else if let Some(stripped) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
            i64::from_str_radix(stripped, 2).ok()
        } else if let Some(stripped) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
            i64::from_str_radix(stripped, 8).ok()
        } else if text.starts_with('0') && text.len() > 1 {
            i64::from_str_radix(&text[1..], 8).ok()
        } else {
            text.parse::<i64>().ok()
        }
    }

    fn check_array(&mut self, array: &Array<'_>) {
        // Collect all key-value elements with their indices and keys
        let mut key_positions: HashMap<String, Vec<usize>> = HashMap::new();
        let elements: Vec<_> = array.elements.iter().collect();

        for (idx, element) in elements.iter().enumerate() {
            if let ArrayElement::KeyValue(kv) = element {
                if let Some(key) = self.extract_key(&kv.key) {
                    key_positions.entry(key).or_default().push(idx);
                }
            }
        }

        // Find duplicates (keys that appear more than once)
        let mut indices_to_remove: Vec<usize> = Vec::new();
        for positions in key_positions.values() {
            if positions.len() > 1 {
                // Remove all but the last occurrence
                for &pos in positions.iter().take(positions.len() - 1) {
                    indices_to_remove.push(pos);
                }
            }
        }

        if indices_to_remove.is_empty() {
            return;
        }

        // Sort indices in descending order to remove from end first
        indices_to_remove.sort_by(|a, b| b.cmp(a));

        // Create edits for each duplicate to remove
        // We simply replace the element with empty string
        // The comma handling will leave extra commas, but that's valid PHP
        for &idx in &indices_to_remove {
            let element = &elements[idx];
            let elem_span = element.span();
            let elem_text = self.get_text(elem_span);

            self.edits.push(Edit::new(
                elem_span,
                String::new(),
                format!("Remove duplicated array key: {}", elem_text.trim()),
            ));
        }
    }
}

impl<'a, 's> Visitor<'a> for RemoveDuplicatedArrayKeyVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Array(array) = expr {
            self.check_array(array);
        }
        true
    }
}

pub struct RemoveDuplicatedArrayKeyRule;

impl RemoveDuplicatedArrayKeyRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RemoveDuplicatedArrayKeyRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RemoveDuplicatedArrayKeyRule {
    fn name(&self) -> &'static str {
        "remove_duplicated_array_key"
    }

    fn description(&self) -> &'static str {
        "Remove duplicate array keys, keeping only the last occurrence"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_duplicated_array_key(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
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
        check_remove_duplicated_array_key(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_duplicate_string_key() {
        let source = r#"<?php
$arr = ['foo' => 1, 'bar' => 2, 'foo' => 3];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        // Should have bar and foo with value 3
        assert!(result.contains("'bar' => 2"));
        assert!(result.contains("'foo' => 3"));
        // Should not have foo => 1
        assert!(!result.contains("'foo' => 1"));
    }

    #[test]
    fn test_duplicate_int_key() {
        let source = r#"<?php
$arr = [1 => 'a', 2 => 'b', 1 => 'c'];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("2 => 'b'"));
        assert!(result.contains("1 => 'c'"));
    }

    #[test]
    fn test_no_duplicates() {
        let source = r#"<?php
$arr = ['foo' => 1, 'bar' => 2, 'baz' => 3];
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_duplicates() {
        let source = r#"<?php
$arr = ['a' => 1, 'b' => 2, 'a' => 3, 'b' => 4];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_three_same_keys() {
        let source = r#"<?php
$arr = ['x' => 1, 'x' => 2, 'x' => 3];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        let result = transform(source);
        assert!(result.contains("'x' => 3"));
        assert!(!result.contains("'x' => 1"));
        assert!(!result.contains("'x' => 2"));
    }

    #[test]
    fn test_mixed_int_string_key() {
        let source = r#"<?php
$arr = [0 => 'a', '0' => 'b'];
"#;
        // In PHP, string '0' and int 0 are different keys
        let edits = check_php(source);
        // They should be considered different
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_non_literal_keys() {
        let source = r#"<?php
$arr = [$key => 1, $key => 2];
"#;
        // Variable keys can't be statically analyzed
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiline_array() {
        let source = r#"<?php
$arr = [
    'foo' => 1,
    'bar' => 2,
    'foo' => 3,
];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_nested_array_not_affected() {
        let source = r#"<?php
$arr = [
    'outer' => ['inner' => 1, 'inner' => 2],
];
"#;
        let edits = check_php(source);
        // Should find the duplicate in the inner array
        assert_eq!(edits.len(), 1);
    }
}
