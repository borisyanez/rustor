//! Fix return type declaration spacing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures proper spacing in return type declarations
pub struct ReturnTypeDeclarationFixer;

impl Fixer for ReturnTypeDeclarationFixer {
    fn name(&self) -> &'static str {
        "return_type_declaration"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "return_type_declaration"
    }

    fn description(&self) -> &'static str {
        "Fix return type declaration spacing"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // PSR-12: No space before colon, one space after
        // function foo() : int -> function foo(): int
        // Handle namespaced types like \App\Model
        let space_before_colon = Regex::new(r"\)\s+:(\s*)(\??)(\\?[\w\\]+)").unwrap();

        for cap in space_before_colon.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let space_after = cap.get(1).unwrap().as_str();
            let nullable = cap.get(2).unwrap().as_str();
            let type_name = cap.get(3).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Need exactly one space after colon
            let new_space = if space_after.is_empty() || space_after.len() > 1 { " " } else { space_after };

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("):{}{}{}", new_space, nullable, type_name),
                "No space before return type colon".to_string(),
                "return_type_declaration",
            ));
        }

        // Fix missing space after colon
        // function foo():int -> function foo(): int
        let no_space_after_colon = Regex::new(r"\):(\??)([A-Za-z\\])").unwrap();

        for cap in no_space_after_colon.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let nullable = cap.get(1).unwrap().as_str();
            let first_char = cap.get(2).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Check if already edited
            let already_edited = edits.iter().any(|e| {
                e.start_offset() <= full_match.start() && e.end_offset() >= full_match.end()
            });

            if !already_edited {
                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    format!("): {}{}", nullable, first_char),
                    "Add space after return type colon".to_string(),
                    "return_type_declaration",
                ));
            }
        }

        // Fix multiple spaces after colon
        // function foo():  int -> function foo(): int
        let multi_space_after_colon = Regex::new(r"\):\s{2,}(\??)(\w)").unwrap();

        for cap in multi_space_after_colon.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let nullable = cap.get(1).unwrap().as_str();
            let first_char = cap.get(2).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            let already_edited = edits.iter().any(|e| {
                e.start_offset() <= full_match.start() && e.end_offset() >= full_match.end()
            });

            if !already_edited {
                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    format!("): {}{}", nullable, first_char),
                    "Single space after return type colon".to_string(),
                    "return_type_declaration",
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
        ReturnTypeDeclarationFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nfunction foo(): int {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_space_before_colon() {
        let source = "<?php\nfunction foo() : int {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("): int"));
    }

    #[test]
    fn test_no_space_after_colon() {
        let source = "<?php\nfunction foo():int {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("): i"));
    }

    #[test]
    fn test_multiple_spaces_after_colon() {
        let source = "<?php\nfunction foo():  int {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("): i"));
    }

    #[test]
    fn test_nullable_type() {
        let source = "<?php\nfunction foo() : ?int {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("): ?int"));
    }

    #[test]
    fn test_class_return_type() {
        let source = "<?php\nfunction foo() : Model {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_namespaced_return_type() {
        let source = "<?php\nfunction foo() : \\App\\Model {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'function foo() : int';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_method() {
        let source = "<?php\nclass A { public function foo() : int {} }\n";
        let edits = check(source);
        assert!(!edits.is_empty());
    }
}
