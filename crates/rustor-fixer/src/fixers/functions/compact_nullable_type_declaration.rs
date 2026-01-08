//! Compact nullable type declaration fixer
//!
//! Removes whitespace after the nullable operator `?` in type declarations.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures no whitespace after `?` in nullable type declarations
pub struct CompactNullableTypeDeclarationFixer;

impl Fixer for CompactNullableTypeDeclarationFixer {
    fn name(&self) -> &'static str {
        "compact_nullable_type_declaration"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "compact_nullable_type_declaration"
    }

    fn description(&self) -> &'static str {
        "Remove whitespace after ? in nullable type declarations"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match `? int`, `? string`, `? array`, `? ClassName`, etc.
        // Must be in type position (after : or before $)
        let re = Regex::new(r"(\?)\s+(\w+)").unwrap();

        for cap in re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let question = cap.get(1).unwrap();
            let type_name = cap.get(2).unwrap().as_str();

            // Skip if in string
            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Skip if in comment
            if is_in_comment(&source[..full_match.start()]) {
                continue;
            }

            // Check context - should be in type position
            let before = &source[..question.start()];
            let trimmed = before.trim_end();

            // Valid contexts: after `:` (return type), after `(` or `,` (param type),
            // or property type declaration
            let valid_context = trimmed.ends_with(':')
                || trimmed.ends_with('(')
                || trimmed.ends_with(',')
                || trimmed.ends_with("public")
                || trimmed.ends_with("private")
                || trimmed.ends_with("protected")
                || trimmed.ends_with("readonly");

            if !valid_context {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("?{}", type_name),
                "Remove whitespace after ? in nullable type".to_string(),
                "compact_nullable_type_declaration",
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
        CompactNullableTypeDeclarationFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nfunction foo(): ?int {}";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_return_type_with_space() {
        let source = "<?php\nfunction foo(): ? int {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "?int");
    }

    #[test]
    fn test_param_type_with_space() {
        let source = "<?php\nfunction foo(? string $bar) {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "?string");
    }

    #[test]
    fn test_property_type_with_space() {
        let source = "<?php\nclass Foo {\n    public ? int $bar;\n}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_ternary() {
        let source = "<?php\n$a = $b ? $c : $d;";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = '? int';";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
