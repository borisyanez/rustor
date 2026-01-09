//! Configure spacing around string concatenation operator

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, ConfigValue, FixerOption, OptionType, edit_with_rule};

/// Configures spacing around the `.` concatenation operator
pub struct ConcatSpaceFixer;

impl Fixer for ConcatSpaceFixer {
    fn name(&self) -> &'static str {
        "concat_space"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "concat_space"
    }

    fn description(&self) -> &'static str {
        "Configure spacing around concatenation operator"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn options(&self) -> Vec<FixerOption> {
        vec![FixerOption {
            name: "spacing",
            description: "Spacing to apply: 'one' (single space) or 'none' (no space)",
            option_type: OptionType::Enum(vec!["one", "none"]),
            default: Some(ConfigValue::String("none".to_string())),
        }]
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let use_space = config.options.get("spacing")
            .and_then(|v| match v {
                ConfigValue::String(s) => Some(s.as_str()),
                _ => None,
            })
            .map(|s| s == "one")
            .unwrap_or(false); // Default: no space (matches PHP-CS-Fixer)

        let mut edits = Vec::new();
        let bytes = source.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'.' {
                // Check if this is a concat operator (not a decimal point or method call)
                if is_concat_operator(source, i) && !is_in_string_or_comment(source, i) {
                    let (space_before, space_after) = get_surrounding_spaces(source, i);

                    if use_space {
                        // Should have exactly one space on each side
                        if space_before != 1 || space_after != 1 {
                            let start = i - space_before;
                            let end = i + 1 + space_after;
                            edits.push(edit_with_rule(
                                start,
                                end,
                                " . ".to_string(),
                                "Use single space around concatenation operator".to_string(),
                                "concat_space",
                            ));
                        }
                    } else {
                        // Should have no spaces
                        if space_before > 0 || space_after > 0 {
                            let start = i - space_before;
                            let end = i + 1 + space_after;
                            edits.push(edit_with_rule(
                                start,
                                end,
                                ".".to_string(),
                                "Remove spaces around concatenation operator".to_string(),
                                "concat_space",
                            ));
                        }
                    }
                }
            }
            i += 1;
        }

        edits
    }
}

/// Check if the dot at position i is a concatenation operator
fn is_concat_operator(source: &str, i: usize) -> bool {
    let bytes = source.as_bytes();

    // Check before: should not be a digit (decimal point) or -> (method call)
    if i > 0 {
        let before = bytes[i - 1];
        if before.is_ascii_digit() {
            // Could be decimal point - check if followed by digit
            if i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
                return false;
            }
        }
        // Check for ->
        if before == b'-' {
            return false;
        }
    }

    // Check if part of .= (compound assignment operator)
    // Skip whitespace to find the next non-space character
    let mut next_idx = i + 1;
    while next_idx < bytes.len() && (bytes[next_idx] == b' ' || bytes[next_idx] == b'\t') {
        next_idx += 1;
    }
    if next_idx < bytes.len() && bytes[next_idx] == b'=' {
        // Check it's not == (comparison after concat)
        if next_idx + 1 >= bytes.len() || bytes[next_idx + 1] != b'=' {
            return false;
        }
    }

    // Check if part of ... (splat operator)
    if i > 0 && i + 1 < bytes.len() {
        if bytes[i - 1] == b'.' || bytes[i + 1] == b'.' {
            return false;
        }
    }

    // Check for ?-> (nullsafe operator)
    if i > 1 && bytes[i - 1] == b'-' && bytes[i - 2] == b'?' {
        return false;
    }

    true
}

/// Get number of spaces before and after position i
fn get_surrounding_spaces(source: &str, i: usize) -> (usize, usize) {
    let bytes = source.as_bytes();

    let mut space_before = 0;
    let mut j = i;
    while j > 0 && (bytes[j - 1] == b' ' || bytes[j - 1] == b'\t') {
        space_before += 1;
        j -= 1;
    }

    let mut space_after = 0;
    let mut k = i + 1;
    while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
        space_after += 1;
        k += 1;
    }

    (space_before, space_after)
}

/// Check if position is inside a string or comment
fn is_in_string_or_comment(source: &str, pos: usize) -> bool {
    let before = &source[..pos];
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        if !in_single_quote && !in_double_quote && !in_block_comment {
            if c == '/' && prev_char == '/' {
                in_line_comment = true;
            }
            if c == '#' {
                in_line_comment = true;
            }
        }

        if !in_single_quote && !in_double_quote && !in_line_comment {
            if c == '*' && prev_char == '/' {
                in_block_comment = true;
            }
            if c == '/' && prev_char == '*' && in_block_comment {
                in_block_comment = false;
            }
        }

        if c == '\n' {
            in_line_comment = false;
        }

        if !in_line_comment && !in_block_comment {
            if c == '\'' && prev_char != '\\' && !in_double_quote {
                in_single_quote = !in_single_quote;
            }
            if c == '"' && prev_char != '\\' && !in_single_quote {
                in_double_quote = !in_double_quote;
            }
        }

        prev_char = c;
    }

    in_single_quote || in_double_quote || in_line_comment || in_block_comment
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn check_no_space(source: &str) -> Vec<Edit> {
        let mut options = HashMap::new();
        options.insert("spacing".to_string(), ConfigValue::String("none".to_string()));
        ConcatSpaceFixer.check(source, &FixerConfig {
            options,
            ..Default::default()
        })
    }

    fn check_with_space(source: &str) -> Vec<Edit> {
        let mut options = HashMap::new();
        options.insert("spacing".to_string(), ConfigValue::String("one".to_string()));
        ConcatSpaceFixer.check(source, &FixerConfig {
            options,
            ..Default::default()
        })
    }

    #[test]
    fn test_no_space_unchanged() {
        let edits = check_no_space("<?php\n$a = 'hello'.'world';\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_remove_spaces() {
        let source = "<?php\n$a = 'hello' . 'world';\n";
        let edits = check_no_space(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, ".");
    }

    #[test]
    fn test_add_spaces() {
        let source = "<?php\n$a = 'hello'.'world';\n";
        let edits = check_with_space(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, " . ");
    }

    #[test]
    fn test_space_already_correct() {
        let source = "<?php\n$a = 'hello' . 'world';\n";
        let edits = check_with_space(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_default_no_spaces() {
        // Default is "none" (no spaces) to match PHP-CS-Fixer
        let source = "<?php\n$a = 'hello'.'world';\n";
        let edits = ConcatSpaceFixer.check(source, &FixerConfig::default());
        // No edits needed when source already has no spaces
        assert!(edits.is_empty());
    }

    #[test]
    fn test_default_removes_spaces() {
        // Default is "none" - should remove existing spaces
        let source = "<?php\n$a = 'hello' . 'world';\n";
        let edits = ConcatSpaceFixer.check(source, &FixerConfig::default());
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, ".");
    }

    #[test]
    fn test_decimal_unchanged() {
        let source = "<?php\n$a = 3.14;\n";
        let edits = check_no_space(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_method_call_unchanged() {
        let source = "<?php\n$a->method();\n";
        let edits = check_no_space(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_concat() {
        let source = "<?php\n$a = 'a' . 'b' . 'c';\n";
        let edits = check_no_space(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'a . b';\n";
        let edits = check_no_space(source);
        assert!(edits.is_empty());
    }
}
