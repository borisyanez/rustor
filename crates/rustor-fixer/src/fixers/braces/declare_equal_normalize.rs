//! Normalize spacing in declare statements

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Normalizes spacing in declare statements
pub struct DeclareEqualNormalizeFixer;

impl Fixer for DeclareEqualNormalizeFixer {
    fn name(&self) -> &'static str {
        "declare_equal_normalize"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "declare_equal_normalize"
    }

    fn description(&self) -> &'static str {
        "Normalize spacing in declare statements"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match declare statements with incorrect spacing around =
        // declare(strict_types = 1) -> declare(strict_types=1)
        // declare( strict_types=1 ) -> declare(strict_types=1)
        let declare_re = Regex::new(r"(?i)\bdeclare\s*\(\s*(\w+)\s*=\s*(\d+)\s*\)").unwrap();

        for cap in declare_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let directive = cap.get(1).unwrap().as_str();
            let value = cap.get(2).unwrap().as_str();

            // Check if it needs fixing (has extra spaces)
            let normalized = format!("declare({}={})", directive, value);
            let current = full_match.as_str();

            if current != normalized {
                if is_in_string(&source[..full_match.start()]) {
                    continue;
                }

                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    normalized,
                    "Normalize declare statement spacing".to_string(),
                    "declare_equal_normalize",
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
        DeclareEqualNormalizeFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\ndeclare(strict_types=1);\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_space_around_equals() {
        let source = "<?php\ndeclare(strict_types = 1);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "declare(strict_types=1)");
    }

    #[test]
    fn test_space_after_paren() {
        let source = "<?php\ndeclare( strict_types=1);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "declare(strict_types=1)");
    }

    #[test]
    fn test_space_before_paren() {
        let source = "<?php\ndeclare(strict_types=1 );\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "declare(strict_types=1)");
    }

    #[test]
    fn test_multiple_spaces() {
        let source = "<?php\ndeclare(  strict_types  =  1  );\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "declare(strict_types=1)");
    }

    #[test]
    fn test_ticks() {
        let source = "<?php\ndeclare(ticks = 1);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "declare(ticks=1)");
    }

    #[test]
    fn test_encoding() {
        let source = "<?php\ndeclare(encoding = 1);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "declare(encoding=1)");
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'declare(strict_types = 1)';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
