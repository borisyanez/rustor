//! Array syntax fixer
//!
//! Converts long array syntax to short array syntax.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Converts array() to [] syntax
pub struct ArraySyntaxFixer;

impl Fixer for ArraySyntaxFixer {
    fn name(&self) -> &'static str {
        "array_syntax"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "array_syntax"
    }

    fn description(&self) -> &'static str {
        "Convert array() to [] syntax"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match array() with contents - need to handle nested arrays
        // Simple approach: match array( and find matching )
        let array_re = Regex::new(r"\barray\s*\(").unwrap();

        for m in array_re.find_iter(source) {
            if is_in_string(&source[..m.start()]) {
                continue;
            }

            if is_in_comment(&source[..m.start()]) {
                continue;
            }

            // Find the matching closing parenthesis
            let start = m.end() - 1; // Position of (
            if let Some(end) = find_matching_paren(source, start) {
                let contents = &source[m.end()..end];

                edits.push(edit_with_rule(
                    m.start(),
                    end + 1,
                    format!("[{}]", contents),
                    "Use short array syntax".to_string(),
                    "array_syntax",
                ));
            }
        }

        edits
    }
}

fn find_matching_paren(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&b'(') {
        return None;
    }

    let mut depth = 1;
    let mut i = start + 1;
    let mut in_string = false;
    let mut string_char = b'\0';

    while i < bytes.len() && depth > 0 {
        let c = bytes[i];

        if in_string {
            if c == string_char && (i == 0 || bytes[i - 1] != b'\\') {
                in_string = false;
            }
        } else {
            match c {
                b'"' | b'\'' => {
                    in_string = true;
                    string_char = c;
                }
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
        }
        i += 1;
    }

    if depth == 0 {
        Some(i - 1)
    } else {
        None
    }
}

fn is_in_string(before: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';
    for c in before.chars() {
        if c == '\'' && prev != '\\' && !in_double { in_single = !in_single; }
        if c == '"' && prev != '\\' && !in_single { in_double = !in_double; }
        prev = c;
    }
    in_single || in_double
}

fn is_in_comment(before: &str) -> bool {
    if let Some(pos) = before.rfind('\n') {
        let line = &before[pos..];
        if line.contains("//") || line.contains('#') { return true; }
    }
    before.matches("/*").count() > before.matches("*/").count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        ArraySyntaxFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_short_syntax_unchanged() {
        let source = "<?php\n$a = [];";
        assert!(check(source).is_empty());
    }

    #[test]
    fn test_empty_array() {
        let source = "<?php\n$a = array();";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("[]"));
    }

    #[test]
    fn test_array_with_values() {
        let source = "<?php\n$a = array(1, 2, 3);";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("[1, 2, 3]"));
    }

    #[test]
    fn test_nested_array() {
        let source = "<?php\n$a = array(array(1));";
        let edits = check(source);
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'array()';";
        assert!(check(source).is_empty());
    }
}
