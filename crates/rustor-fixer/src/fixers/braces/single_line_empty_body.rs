//! Single line empty body fixer
//!
//! Ensures empty function/method bodies are on a single line.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures empty bodies are written on a single line
pub struct SingleLineEmptyBodyFixer;

impl Fixer for SingleLineEmptyBodyFixer {
    fn name(&self) -> &'static str {
        "single_line_empty_body"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "single_line_empty_body"
    }

    fn description(&self) -> &'static str {
        "Ensure empty function/method bodies are on a single line"
    }

    fn priority(&self) -> i32 {
        25
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match multiline empty bodies: {\n  \n} or {\n}
        // Pattern: { followed by whitespace/newlines only, then }
        let re = Regex::new(r"\{\s*\n\s*\}").unwrap();

        for cap in re.find_iter(source) {
            let match_str = cap.as_str();

            // Skip if in string
            if is_in_string(&source[..cap.start()]) {
                continue;
            }

            // Check if this is truly empty (only whitespace between braces)
            let inner = &match_str[1..match_str.len() - 1];
            if inner.trim().is_empty() {
                edits.push(edit_with_rule(
                    cap.start(),
                    cap.end(),
                    "{}".to_string(),
                    "Empty body should be on single line".to_string(),
                    "single_line_empty_body",
                ));
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
        SingleLineEmptyBodyFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nfunction foo() {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiline_empty() {
        let source = "<?php\nfunction foo() {\n}\n";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "{}");
    }

    #[test]
    fn test_multiline_empty_with_whitespace() {
        let source = "<?php\nfunction foo() {\n    \n}\n";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "{}");
    }

    #[test]
    fn test_non_empty_unchanged() {
        let source = "<?php\nfunction foo() {\n    return 1;\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_class_empty_body() {
        let source = "<?php\nclass Foo {\n}\n";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }
}
