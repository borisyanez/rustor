//! New with parentheses fixer
//!
//! Ensures `new Foo` is written as `new Foo()`.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures new expressions include parentheses
pub struct NewWithParenthesesFixer;

impl Fixer for NewWithParenthesesFixer {
    fn name(&self) -> &'static str {
        "new_with_parentheses"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "new_with_parentheses"
    }

    fn description(&self) -> &'static str {
        "Ensure 'new Foo' is written as 'new Foo()'"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match `new ClassName` - we'll check for parentheses manually
        let re = Regex::new(r"\bnew\s+([A-Z][a-zA-Z0-9_\\]*)").unwrap();

        for cap in re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let class_name = cap.get(1).unwrap();

            // Skip if in string
            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Skip if in comment
            if is_in_comment(&source[..full_match.start()]) {
                continue;
            }

            // Skip anonymous classes: new class { }
            if class_name.as_str().to_lowercase() == "class" {
                continue;
            }

            // Check what comes after - skip if it's already has ()
            let after = &source[full_match.end()..];
            let after_trimmed = after.trim_start();
            if after_trimmed.starts_with('(') {
                continue;
            }

            // Skip if followed by :: (static access on instantiated object)
            if after_trimmed.starts_with("::") {
                continue;
            }

            edits.push(edit_with_rule(
                class_name.end(),
                class_name.end(),
                "()".to_string(),
                format!("Add parentheses to 'new {}'", class_name.as_str()),
                "new_with_parentheses",
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
    if let Some(last_line_start) = before.rfind('\n') {
        let last_line = &before[last_line_start..];
        if last_line.contains("//") || last_line.contains('#') {
            return true;
        }
    } else if before.contains("//") {
        return true;
    }

    let open_count = before.matches("/*").count();
    let close_count = before.matches("*/").count();
    open_count > close_count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        NewWithParenthesesFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\n$a = new Foo();";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_with_args_unchanged() {
        let source = "<?php\n$a = new Foo($bar);";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_missing_parentheses() {
        let source = "<?php\n$a = new Foo;";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "()");
    }

    #[test]
    fn test_namespaced_class() {
        let source = "<?php\n$a = new App\\Model\\User;";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_anonymous_class() {
        let source = "<?php\n$a = new class { };";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'new Foo';";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_with_space_before_paren() {
        let source = "<?php\n$a = new Foo ();";
        let edits = check(source);
        assert!(edits.is_empty()); // Already has parens
    }
}
