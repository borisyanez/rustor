//! Single quote fixer
//!
//! Converts double quotes to single quotes where possible.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Converts double-quoted strings to single-quoted where possible
pub struct SingleQuoteFixer;

impl Fixer for SingleQuoteFixer {
    fn name(&self) -> &'static str {
        "single_quote"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "single_quote"
    }

    fn description(&self) -> &'static str {
        "Convert double quotes to single quotes where possible"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match double-quoted strings that don't contain:
        // - Variables ($var)
        // - Escape sequences other than \\ and \"
        let double_quote_re = Regex::new(r#""([^"\\]*(?:\\.[^"\\]*)*)""#).unwrap();

        for cap in double_quote_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let content = cap.get(1).unwrap().as_str();

            // Skip if contains variable interpolation
            if content.contains('$') {
                continue;
            }

            // Skip if contains escape sequences other than \\ and \"
            if has_special_escapes(content) {
                continue;
            }

            // Convert \" to ' and \\ to \
            let converted = content
                .replace("\\\"", "\"")
                .replace("\\'", "\\'")  // Keep escaped single quotes
                .replace("'", "\\'");   // Escape any single quotes

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("'{}'", converted),
                "Use single quotes for simple strings".to_string(),
                "single_quote",
            ));
        }

        edits
    }
}

fn has_special_escapes(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            // Allow \\ and \" only
            if next != b'\\' && next != b'"' {
                return true;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        SingleQuoteFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_single_quote_unchanged() {
        let source = "<?php\n$a = 'hello';";
        assert!(check(source).is_empty());
    }

    #[test]
    fn test_simple_double_quote() {
        let source = r#"<?php
$a = "hello";"#;
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("'hello'"));
    }

    #[test]
    fn test_skip_variable() {
        let source = r#"<?php
$a = "hello $name";"#;
        assert!(check(source).is_empty());
    }

    #[test]
    fn test_skip_newline_escape() {
        let source = r#"<?php
$a = "hello\nworld";"#;
        assert!(check(source).is_empty());
    }

    #[test]
    fn test_skip_tab_escape() {
        let source = r#"<?php
$a = "hello\tworld";"#;
        assert!(check(source).is_empty());
    }

    #[test]
    fn test_allow_escaped_quote() {
        let source = r#"<?php
$a = "say \"hello\"";"#;
        let edits = check(source);
        assert!(!edits.is_empty());
    }
}
