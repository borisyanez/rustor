//! Ensure proper spacing in switch case statements

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures no space before colon in case/default statements
pub struct SwitchCaseSpaceFixer;

impl Fixer for SwitchCaseSpaceFixer {
    fn name(&self) -> &'static str {
        "switch_case_space"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "switch_case_space"
    }

    fn description(&self) -> &'static str {
        "Remove space before colon in case/default statements"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match case VALUE : (with space before colon)
        // case 'value' :  ->  case 'value':
        let case_re = Regex::new(r"(?i)\b(case\s+.+?)\s+:").unwrap();

        for cap in case_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let case_part = cap.get(1).unwrap();

            // Skip if in string or comment
            if is_in_string_or_comment(&source[..full_match.start()]) {
                continue;
            }

            // Replace with no space before colon
            let replacement = format!("{}:", case_part.as_str());

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                "Remove space before colon in case statement".to_string(),
                "switch_case_space",
            ));
        }

        // Match default : (with space before colon)
        let default_re = Regex::new(r"(?i)\b(default)\s+:").unwrap();

        for cap in default_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let default_part = cap.get(1).unwrap();

            if is_in_string_or_comment(&source[..full_match.start()]) {
                continue;
            }

            let replacement = format!("{}:", default_part.as_str());

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                "Remove space before colon in default statement".to_string(),
                "switch_case_space",
            ));
        }

        edits
    }
}

fn is_in_string_or_comment(before: &str) -> bool {
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

    fn check(source: &str) -> Vec<Edit> {
        SwitchCaseSpaceFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let edits = check("<?php\nswitch ($a) {\n    case 1:\n        break;\n}\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_space_before_colon() {
        let source = "<?php\nswitch ($a) {\n    case 1 :\n        break;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.ends_with(':'));
        assert!(!edits[0].replacement.ends_with(" :"));
    }

    #[test]
    fn test_default_space() {
        let source = "<?php\nswitch ($a) {\n    default :\n        break;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "default:");
    }

    #[test]
    fn test_string_case() {
        let source = "<?php\nswitch ($a) {\n    case 'test' :\n        break;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_cases() {
        let source = "<?php\nswitch ($a) {\n    case 1 :\n    case 2 :\n        break;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }
}
