//! Ternary operator spaces fixer
//!
//! Ensures proper spacing around `?` and `:` in ternary expressions.
//! This is a conservative implementation that only flags obvious cases.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures proper spacing around ternary operator
pub struct TernaryOperatorSpacesFixer;

impl Fixer for TernaryOperatorSpacesFixer {
    fn name(&self) -> &'static str {
        "ternary_operator_spaces"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "ternary_operator_spaces"
    }

    fn description(&self) -> &'static str {
        "Ensure proper spacing around ternary operator ? and :"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // This is a conservative implementation.
        // Detecting ternary operators reliably requires parsing context.
        // We'll look for patterns that are clearly ternary operators.

        // Pattern: $var ? $a : $b with missing spaces
        // Look for: ) ? without space before or after, or : without space

        // Match patterns like `$x?$y` (variable immediately followed by ? then variable)
        // This is clearly a ternary, not nullable type
        let var_q_var = Regex::new(r"\$\w+\?(\$\w+)").unwrap();
        for cap in var_q_var.captures_iter(source) {
            let full_match = cap.get(0).unwrap();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Find the ? position
            let q_pos = full_match.start() + full_match.as_str().find('?').unwrap();

            edits.push(edit_with_rule(
                q_pos,
                q_pos + 1,
                " ? ".to_string(),
                "Add space around ternary ?".to_string(),
                "ternary_operator_spaces",
            ));
        }

        // Match patterns like `true?` or `false?` followed by non-space
        let bool_q = Regex::new(r"\b(true|false)\?([^\s>?])").unwrap();
        for cap in bool_q.captures_iter(source) {
            let full_match = cap.get(0).unwrap();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            let keyword = cap.get(1).unwrap();
            let q_pos = keyword.end();

            edits.push(edit_with_rule(
                q_pos,
                q_pos + 1,
                " ? ".to_string(),
                "Add space around ternary ?".to_string(),
                "ternary_operator_spaces",
            ));
        }

        // Match closing paren followed by ?non-space (clearly ternary after condition)
        let paren_q = Regex::new(r"\)\?([^\s>?])").unwrap();
        for cap in paren_q.captures_iter(source) {
            let full_match = cap.get(0).unwrap();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            let q_pos = full_match.start() + 1;

            edits.push(edit_with_rule(
                q_pos,
                q_pos + 1,
                " ? ".to_string(),
                "Add space around ternary ?".to_string(),
                "ternary_operator_spaces",
            ));
        }

        // Handle ternary colon :
        // Pattern: `? expr:expr` or `? expr :expr` - colon without proper spacing
        // We look for `?` followed by stuff then `:` without spaces around it
        // This is tricky because : is used in many contexts (case:, labels:, ::, etc.)
        //
        // Safe pattern: variable or value followed by :$ or :non-space
        // We need to be in a ternary context (after a ?)

        // Look for ? ... : patterns where : needs spacing
        // Pattern: find `? anything :something` where : has no space before or after
        let ternary_colon = Regex::new(r"\?\s*[^:?]+:([^\s:])").unwrap();
        for cap in ternary_colon.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let match_str = full_match.as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Find the : position
            if let Some(colon_offset) = match_str.rfind(':') {
                let colon_pos = full_match.start() + colon_offset;

                // Check if there's space before the colon
                let prev_char = source.chars().nth(colon_pos.saturating_sub(1));
                let has_space_before = prev_char.map(|c| c.is_whitespace()).unwrap_or(false);

                if has_space_before {
                    // Only need space after
                    edits.push(edit_with_rule(
                        colon_pos,
                        colon_pos + 1,
                        ": ".to_string(),
                        "Add space after ternary :".to_string(),
                        "ternary_operator_spaces",
                    ));
                } else {
                    // Need space before and after
                    edits.push(edit_with_rule(
                        colon_pos,
                        colon_pos + 1,
                        " : ".to_string(),
                        "Add space around ternary :".to_string(),
                        "ternary_operator_spaces",
                    ));
                }
            }
        }

        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        if c == '\'' && prev_char != '\\' && !in_double_quote {
            in_single_quote = !in_single_quote;
        }
        if c == '"' && prev_char != '\\' && !in_single_quote {
            in_double_quote = !in_double_quote;
        }
        prev_char = c;
    }

    in_single_quote || in_double_quote
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        TernaryOperatorSpacesFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\n$a = $b ? $c : $d;";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_var_no_space_around_question() {
        let source = "<?php\n$a = $b?$c : $d;";
        let edits = check(source);
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_skip_nullable_type() {
        let source = "<?php\nfunction foo(): ?int {}";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_nullsafe() {
        let source = "<?php\n$a?->foo();";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_null_coalescing() {
        let source = "<?php\n$a ?? $b;";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = '$b?$c:$d';";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_paren_then_question() {
        let source = "<?php\n$a = ($b)?$c : $d;";
        let edits = check(source);
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_bool_then_question() {
        let source = "<?php\n$a = true?1 : 0;";
        let edits = check(source);
        assert!(!edits.is_empty());
    }
}
