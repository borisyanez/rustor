//! Convert case/default semicolons to colons

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Converts semicolons in case/default statements to colons
pub struct SwitchCaseSemicolonToColonFixer;

impl Fixer for SwitchCaseSemicolonToColonFixer {
    fn name(&self) -> &'static str {
        "switch_case_semicolon_to_colon"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "switch_case_semicolon_to_colon"
    }

    fn description(&self) -> &'static str {
        "Convert case/default semicolons to colons"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match case statements with semicolon instead of colon
        // case 1; -> case 1:
        // case 'a'; -> case 'a':
        // case CONST; -> case CONST:
        let case_re = Regex::new(r"(?i)\bcase\s+([^;:]+);").unwrap();

        for cap in case_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let value = cap.get(1).unwrap().as_str().trim();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("case {}:", value),
                "Use colon instead of semicolon in case statement".to_string(),
                "switch_case_semicolon_to_colon",
            ));
        }

        // Match default with semicolon
        // default; -> default:
        let default_re = Regex::new(r"(?i)\bdefault\s*;").unwrap();

        for mat in default_re.find_iter(source) {
            if is_in_string(&source[..mat.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                mat.start(),
                mat.end(),
                "default:".to_string(),
                "Use colon instead of semicolon in default statement".to_string(),
                "switch_case_semicolon_to_colon",
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
        SwitchCaseSemicolonToColonFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nswitch ($a) { case 1: break; }\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_case_semicolon() {
        let source = "<?php\nswitch ($a) { case 1; break; }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("case 1:"));
    }

    #[test]
    fn test_case_string_semicolon() {
        let source = "<?php\nswitch ($a) { case 'foo'; break; }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("case 'foo':"));
    }

    #[test]
    fn test_default_semicolon() {
        let source = "<?php\nswitch ($a) { default; break; }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "default:");
    }

    #[test]
    fn test_multiple_cases() {
        let source = "<?php\nswitch ($a) { case 1; case 2; default; }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 3);
    }

    #[test]
    fn test_case_constant() {
        let source = "<?php\nswitch ($a) { case SOME_CONST; break; }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("case SOME_CONST:"));
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'case 1;';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
