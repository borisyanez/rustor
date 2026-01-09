//! Spaces inside parentheses fixer

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Remove or add spaces inside parentheses
pub struct SpacesInsideParenthesesFixer;

impl Fixer for SpacesInsideParenthesesFixer {
    fn name(&self) -> &'static str { "spaces_inside_parentheses" }
    fn php_cs_fixer_name(&self) -> &'static str { "spaces_inside_parentheses" }
    fn description(&self) -> &'static str { "Remove spaces inside parentheses" }
    fn priority(&self) -> i32 { 35 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Remove space after opening paren: ( $a → ($a
        // Match "( " followed by non-whitespace
        let open_space_re = Regex::new(r"\(\s+([^\s\)])").unwrap();
        for cap in open_space_re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let after = cap.get(1).unwrap().as_str();

            // Skip if in string context (basic check)
            if is_in_string(&source[..full.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full.start(),
                full.end(),
                format!("({}", after),
                "Remove space after opening parenthesis".to_string(),
                "spaces_inside_parentheses",
            ));
        }

        // Remove space before closing paren: $a ) → $a)
        // Match non-whitespace followed by " )"
        let close_space_re = Regex::new(r"([^\s\(])\s+\)").unwrap();
        for cap in close_space_re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let before = cap.get(1).unwrap().as_str();

            // Skip if in string context
            if is_in_string(&source[..full.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full.start(),
                full.end(),
                format!("{})", before),
                "Remove space before closing parenthesis".to_string(),
                "spaces_inside_parentheses",
            ));
        }

        // Handle empty parens with spaces: ( ) → ()
        let empty_re = Regex::new(r"\(\s+\)").unwrap();
        for cap in empty_re.captures_iter(source) {
            let full = cap.get(0).unwrap();

            if is_in_string(&source[..full.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full.start(),
                full.end(),
                "()".to_string(),
                "Remove spaces in empty parentheses".to_string(),
                "spaces_inside_parentheses",
            ));
        }

        edits
    }
}

/// Basic check if position is inside a string
fn is_in_string(text: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = ' ';

    for c in text.chars() {
        if c == '\'' && prev_char != '\\' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && prev_char != '\\' && !in_single_quote {
            in_double_quote = !in_double_quote;
        }
        prev_char = c;
    }

    in_single_quote || in_double_quote
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_space_after_open() {
        let code = "<?php\nif( $a ) {}";
        let edits = SpacesInsideParenthesesFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_space_before_close() {
        let code = "<?php\nif($a ) {}";
        let edits = SpacesInsideParenthesesFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_no_spaces() {
        let code = "<?php\nif($a) {}";
        let edits = SpacesInsideParenthesesFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_parens_with_space() {
        let code = "<?php\nfoo( );";
        let edits = SpacesInsideParenthesesFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_both_sides() {
        let code = "<?php\nif ( $a && $b ) {}";
        let edits = SpacesInsideParenthesesFixer.check(code, &FixerConfig::default());
        // Should have edits for both sides
        assert!(edits.len() >= 2);
    }
}
