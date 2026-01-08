//! Array indentation fixer
//!
//! Ensures multiline arrays have consistent indentation.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures consistent indentation in multiline arrays
pub struct ArrayIndentationFixer;

impl Fixer for ArrayIndentationFixer {
    fn name(&self) -> &'static str {
        "array_indentation"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "array_indentation"
    }

    fn description(&self) -> &'static str {
        "Ensure multiline arrays have consistent indentation"
    }

    fn priority(&self) -> i32 {
        25
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let indent_str = config.indent.as_str();
        let lines: Vec<&str> = source.lines().collect();

        // Track array depth and expected indent
        let mut in_array_stack: Vec<usize> = Vec::new(); // stack of base indents
        let mut offset = 0;

        for (line_num, line) in lines.iter().enumerate() {
            let line_start = offset;
            let trimmed = line.trim_start();
            let current_indent = line.len() - trimmed.len();

            // Skip empty lines
            if trimmed.is_empty() {
                offset += line.len() + 1;
                continue;
            }

            // Check for array opening
            if trimmed.contains('[') && !is_in_string_simple(line) {
                // Count opening and closing brackets
                let opens = trimmed.matches('[').count();
                let closes = trimmed.matches(']').count();

                if opens > closes {
                    // Array opened but not closed on same line
                    in_array_stack.push(current_indent);
                }
            }

            // Check for array closing
            if !in_array_stack.is_empty() && trimmed.starts_with(']') {
                let expected_indent = in_array_stack.last().copied().unwrap_or(0);

                if current_indent != expected_indent {
                    // Fix indentation of closing bracket
                    let expected_ws = " ".repeat(expected_indent);
                    edits.push(edit_with_rule(
                        line_start,
                        line_start + current_indent,
                        expected_ws,
                        "Fix array closing bracket indentation".to_string(),
                        "array_indentation",
                    ));
                }

                in_array_stack.pop();
            } else if !in_array_stack.is_empty() {
                // Inside an array - check element indentation
                let base_indent = in_array_stack.last().copied().unwrap_or(0);
                let expected_indent = base_indent + indent_str.len();

                // Only fix if we're clearly an array element (starts with value, key, or comma continuation)
                if !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with("*") {
                    if current_indent != expected_indent && current_indent != base_indent {
                        // Check if this line is a continuation or new element
                        let expected_ws = " ".repeat(expected_indent);
                        edits.push(edit_with_rule(
                            line_start,
                            line_start + current_indent,
                            expected_ws,
                            "Fix array element indentation".to_string(),
                            "array_indentation",
                        ));
                    }
                }
            }

            // Handle closing bracket in middle of line
            if trimmed.contains(']') && !trimmed.starts_with(']') {
                let closes = trimmed.matches(']').count();
                let opens = trimmed.matches('[').count();
                for _ in 0..(closes.saturating_sub(opens)) {
                    in_array_stack.pop();
                }
            }

            offset += line.len() + 1;
        }

        edits
    }
}

fn is_in_string_simple(line: &str) -> bool {
    // Simple check - if line has unbalanced quotes, might be in string
    let single_quotes = line.matches('\'').count();
    let double_quotes = line.matches('"').count();
    single_quotes % 2 != 0 || double_quotes % 2 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        ArrayIndentationFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\n$arr = [\n    'a',\n    'b',\n];\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_single_line_unchanged() {
        let source = "<?php\n$arr = ['a', 'b', 'c'];\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
