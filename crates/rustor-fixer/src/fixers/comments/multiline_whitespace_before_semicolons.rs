//! Handle whitespace before semicolons in multiline statements

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes or ensures proper whitespace before semicolons
pub struct MultilineWhitespaceBeforeSemicolonsFixer;

impl Fixer for MultilineWhitespaceBeforeSemicolonsFixer {
    fn name(&self) -> &'static str {
        "multiline_whitespace_before_semicolons"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "multiline_whitespace_before_semicolons"
    }

    fn description(&self) -> &'static str {
        "Remove whitespace before semicolons in multiline statements"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match whitespace (including newlines) before semicolons
        // But skip cases in strings
        let re = Regex::new(r"[ \t]+;").unwrap();

        for mat in re.find_iter(source) {
            let before = &source[..mat.start()];

            // Skip if in string
            if is_in_string(before) {
                continue;
            }

            // Skip if in for loop header (for ($i = 0; $i < 10; $i++))
            // Check for unbalanced parentheses
            let paren_depth = count_parens(before);
            if paren_depth > 0 {
                continue;
            }

            edits.push(edit_with_rule(
                mat.start(),
                mat.end(),
                ";".to_string(),
                "Remove space before semicolon".to_string(),
                "multiline_whitespace_before_semicolons",
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

fn count_parens(s: &str) -> i32 {
    let mut depth: i32 = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for c in s.chars() {
        if c == '\'' && prev_char != '\\' && !in_double_quote {
            in_single_quote = !in_single_quote;
        }
        if c == '"' && prev_char != '\\' && !in_single_quote {
            in_double_quote = !in_double_quote;
        }

        if !in_single_quote && !in_double_quote {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }

        prev_char = c;
    }

    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        MultilineWhitespaceBeforeSemicolonsFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\n$a = 1;\n$b = 2;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_space_before_semicolon() {
        let source = "<?php\n$a = 1 ;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, ";");
    }

    #[test]
    fn test_multiple_spaces() {
        let source = "<?php\n$a = 1   ;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_tab_before_semicolon() {
        let source = "<?php\n$a = 1\t;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'test ;';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_for_loop() {
        // Spaces in for loop header should be preserved
        let source = "<?php\nfor ($i = 0 ; $i < 10 ; $i++) {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
