//! Fix spacing in method/function arguments

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures proper spacing around function/method arguments
pub struct MethodArgumentSpaceFixer;

impl Fixer for MethodArgumentSpaceFixer {
    fn name(&self) -> &'static str {
        "method_argument_space"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "method_argument_space"
    }

    fn description(&self) -> &'static str {
        "Fix spacing in method arguments"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Fix space after opening parenthesis in function calls/definitions
        // foo( $a) -> foo($a)
        let space_after_open = Regex::new(r"(\w+)\([ \t]+([^\s\)])").unwrap();

        for cap in space_after_open.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let func = cap.get(1).unwrap().as_str();
            let first_char = cap.get(2).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{}({}", func, first_char),
                "Remove space after opening parenthesis".to_string(),
                "method_argument_space",
            ));
        }

        // Fix space before closing parenthesis
        // foo($a ) -> foo($a)
        let space_before_close = Regex::new(r"([^\s\(])[ \t]+\)").unwrap();

        for cap in space_before_close.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let last_char = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Skip array syntax [ ]
            if last_char == "]" || last_char == "[" {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{})", last_char),
                "Remove space before closing parenthesis".to_string(),
                "method_argument_space",
            ));
        }

        // Fix missing space after comma
        // foo($a,$b) -> foo($a, $b)
        let no_space_after_comma = Regex::new(r",([^\s\n])").unwrap();

        for cap in no_space_after_comma.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let next_char = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Skip if in array (we're only fixing function args)
            // This is a simplification - we'd need context tracking for accuracy

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!(", {}", next_char),
                "Add space after comma".to_string(),
                "method_argument_space",
            ));
        }

        // Fix multiple spaces after comma
        // foo($a,  $b) -> foo($a, $b)
        let multi_space_after_comma = Regex::new(r",[ \t]{2,}(\S)").unwrap();

        for cap in multi_space_after_comma.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let next_char = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!(", {}", next_char),
                "Single space after comma".to_string(),
                "method_argument_space",
            ));
        }

        // Fix space before comma
        // foo($a , $b) -> foo($a, $b)
        let space_before_comma = Regex::new(r"(\S)[ \t]+,").unwrap();

        for cap in space_before_comma.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let prev_char = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{},", prev_char),
                "Remove space before comma".to_string(),
                "method_argument_space",
            ));
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
        MethodArgumentSpaceFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nfoo($a, $b, $c);\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_space_after_open() {
        let source = "<?php\nfoo( $a);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("foo($"));
    }

    #[test]
    fn test_space_before_close() {
        let source = "<?php\nfoo($a );\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.ends_with(")"));
    }

    #[test]
    fn test_no_space_after_comma() {
        let source = "<?php\nfoo($a,$b);\n";
        let edits = check(source);

        assert!(edits.iter().any(|e| e.replacement.contains(", ")));
    }

    #[test]
    fn test_multiple_spaces_after_comma() {
        let source = "<?php\nfoo($a,  $b);\n";
        let edits = check(source);

        assert!(edits.iter().any(|e| e.replacement == ", $"));
    }

    #[test]
    fn test_space_before_comma() {
        let source = "<?php\nfoo($a , $b);\n";
        let edits = check(source);

        // Regex captures single char before space, so replacement is "a,"
        assert!(edits.iter().any(|e| e.replacement == "a,"));
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'foo( $a )';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_function_definition() {
        let source = "<?php\nfunction foo( $a , $b ) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
    }
}
