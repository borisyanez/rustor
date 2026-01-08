//! No unneeded braces fixer
//!
//! Removes unnecessary curly braces that don't define a scope.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes unnecessary curly braces
pub struct NoUnneededBracesFixer;

impl Fixer for NoUnneededBracesFixer {
    fn name(&self) -> &'static str {
        "no_unneeded_braces"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_unneeded_braces"
    }

    fn description(&self) -> &'static str {
        "Remove unnecessary curly braces"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match standalone braces that aren't part of a control structure
        // Pattern: statement; { ... } where the braces don't follow a keyword
        // This is tricky - we look for `{ }` that follow a semicolon directly

        // Match `; { }` pattern (empty braces after statement)
        let empty_braces_re = Regex::new(r";\s*\{\s*\}").unwrap();
        for m in empty_braces_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }

            // Find the start of the braces (after semicolon)
            let brace_start = m.start() + m.as_str().find('{').unwrap();

            edits.push(edit_with_rule(
                brace_start,
                m.end(),
                "".to_string(),
                "Remove unnecessary empty braces".to_string(),
                "no_unneeded_braces",
            ));
        }

        // Match switch case with unnecessary braces around single statement
        // case 1: { break; } -> case 1: break;
        let case_braces_re = Regex::new(r"(case\s+[^:]+:\s*)\{\s*([^{}]+)\s*\}").unwrap();
        for cap in case_braces_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            let case_part = cap.get(1).unwrap().as_str();
            let inner = cap.get(2).unwrap().as_str().trim();

            // Only if inner is a simple statement (ends with ; or break/return)
            if inner.ends_with(';') || inner.ends_with("break;") || inner.ends_with("return;") {
                // Check it's not multiple statements
                let semicolons = inner.matches(';').count();
                if semicolons <= 2 {  // Allow break; or single statement + break;
                    edits.push(edit_with_rule(
                        full_match.start(),
                        full_match.end(),
                        format!("{}{}", case_part, inner),
                        "Remove unnecessary braces in case statement".to_string(),
                        "no_unneeded_braces",
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
        NoUnneededBracesFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nif ($a) { foo(); }";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_braces_after_statement() {
        let source = "<?php\n$a = 1; { }";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "");
    }

    #[test]
    fn test_case_with_braces() {
        let source = "<?php\nswitch ($a) {\n    case 1: { break; }\n}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = '{ }';";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
