//! Convert loose comparisons to strict comparisons
//!
//! This is a risky fixer because changing `==` to `===` can break code
//! that relies on PHP's type juggling.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Converts loose comparisons (==, !=) to strict comparisons (===, !==)
pub struct StrictComparisonFixer;

impl Fixer for StrictComparisonFixer {
    fn name(&self) -> &'static str {
        "strict_comparison"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "strict_comparison"
    }

    fn description(&self) -> &'static str {
        "Convert loose comparisons to strict comparisons"
    }

    fn priority(&self) -> i32 {
        5  // Low priority - runs after most other fixers
    }

    fn is_risky(&self) -> bool {
        true
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match == that is not === (negative lookbehind/lookahead simulation)
        // We need to be careful not to match ===
        let eq_re = Regex::new(r"([^=!<>])(\s*==\s*)([^=])").unwrap();

        for cap in eq_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let before = cap.get(1).unwrap().as_str();
            let op_match = cap.get(2).unwrap();
            let after = cap.get(3).unwrap().as_str();

            // Skip if already strict or assignment
            let op = op_match.as_str().trim();
            if op == "===" || op == "==" && before.ends_with('=') {
                continue;
            }

            // Skip if in string
            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Skip if in comment
            if is_in_comment(&source[..full_match.start()]) {
                continue;
            }

            // Get the spacing from original
            let has_space_before = op_match.as_str().starts_with(' ');
            let has_space_after = op_match.as_str().ends_with(' ');
            let new_op = format!(
                "{}==={}",
                if has_space_before { " " } else { "" },
                if has_space_after { " " } else { "" }
            );

            edits.push(edit_with_rule(
                op_match.start(),
                op_match.end(),
                new_op,
                "Use strict comparison (===) instead of loose comparison (==)".to_string(),
                "strict_comparison",
            ));
        }

        // Match != that is not !==
        let neq_re = Regex::new(r"(\s*!=\s*)([^=])").unwrap();

        for cap in neq_re.captures_iter(source) {
            let op_match = cap.get(1).unwrap();
            let after_char = cap.get(2).unwrap().as_str();

            // Skip if already strict
            if after_char == "=" {
                continue;
            }

            // Skip if in string
            if is_in_string(&source[..op_match.start()]) {
                continue;
            }

            // Skip if in comment
            if is_in_comment(&source[..op_match.start()]) {
                continue;
            }

            // Get the spacing from original
            let has_space_before = op_match.as_str().starts_with(' ');
            let has_space_after = op_match.as_str().ends_with(' ');
            let new_op = format!(
                "{}!=={}",
                if has_space_before { " " } else { "" },
                if has_space_after { " " } else { "" }
            );

            edits.push(edit_with_rule(
                op_match.start(),
                op_match.end(),
                new_op,
                "Use strict comparison (!==) instead of loose comparison (!=)".to_string(),
                "strict_comparison",
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

fn is_in_comment(before: &str) -> bool {
    // Check for single-line comment
    if let Some(last_line_start) = before.rfind('\n') {
        let last_line = &before[last_line_start..];
        if last_line.contains("//") || last_line.contains('#') {
            return true;
        }
    } else if before.contains("//") || before.contains('#') {
        return true;
    }

    // Check for multi-line comment (not closed)
    let open_count = before.matches("/*").count();
    let close_count = before.matches("*/").count();
    open_count > close_count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        StrictComparisonFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_strict_already_unchanged() {
        let source = "<?php\nif ($a === $b) {}";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_loose_to_strict_eq() {
        let source = "<?php\nif ($a == $b) {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("==="));
    }

    #[test]
    fn test_loose_to_strict_neq() {
        let source = "<?php\nif ($a != $b) {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("!=="));
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'if ($x == $y)';";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_comment() {
        let source = "<?php\n// if ($a == $b)";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_is_risky() {
        assert!(StrictComparisonFixer.is_risky());
    }
}
