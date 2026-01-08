//! No space around double colon fixer
//!
//! Removes whitespace around the `::` operator.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes whitespace around the double colon operator
pub struct NoSpaceAroundDoubleColonFixer;

impl Fixer for NoSpaceAroundDoubleColonFixer {
    fn name(&self) -> &'static str {
        "no_space_around_double_colon"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_space_around_double_colon"
    }

    fn description(&self) -> &'static str {
        "Remove whitespace around :: operator"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match space before ::
        // Foo :: bar -> Foo::bar
        let before_re = Regex::new(r"(\w)\s+::").unwrap();
        for cap in before_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            if is_in_comment(&source[..full_match.start()]) {
                continue;
            }

            let char_match = cap.get(1).unwrap();
            edits.push(edit_with_rule(
                char_match.end(),
                full_match.end() - 2,  // Keep the ::
                "".to_string(),
                "Remove space before ::".to_string(),
                "no_space_around_double_colon",
            ));
        }

        // Match space after ::
        // Foo:: bar -> Foo::bar
        let after_re = Regex::new(r"::\s+(\w)").unwrap();
        for cap in after_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            if is_in_comment(&source[..full_match.start()]) {
                continue;
            }

            let char_match = cap.get(1).unwrap();
            edits.push(edit_with_rule(
                full_match.start() + 2,  // After the ::
                char_match.start(),
                "".to_string(),
                "Remove space after ::".to_string(),
                "no_space_around_double_colon",
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
        NoSpaceAroundDoubleColonFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nFoo::bar();";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_space_before() {
        let source = "<?php\nFoo ::bar();";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_space_after() {
        let source = "<?php\nFoo:: bar();";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_space_both() {
        let source = "<?php\nFoo :: bar();";
        let edits = check(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_class_constant() {
        let source = "<?php\nFoo :: CONST;";
        let edits = check(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_self_static() {
        let source = "<?php\nself :: method();";
        let edits = check(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'Foo :: bar';";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
