//! No whitespace before semicolons

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoSinglelineWhitespaceBeforeSemicolonsFixer;

impl Fixer for NoSinglelineWhitespaceBeforeSemicolonsFixer {
    fn name(&self) -> &'static str { "no_singleline_whitespace_before_semicolons" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_singleline_whitespace_before_semicolons" }
    fn description(&self) -> &'static str { "No space before semicolon" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match space(s) before semicolon (not newlines)
        let re = Regex::new(r"[ \t]+;").unwrap();

        for m in re.find_iter(source) {
            if is_in_string(&source[..m.start()]) { continue; }

            edits.push(edit_with_rule(
                m.start(), m.end(),
                ";".to_string(),
                "Remove space before semicolon".to_string(),
                "no_singleline_whitespace_before_semicolons",
            ));
        }

        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let (mut s, mut d, mut p) = (false, false, '\0');
    for c in before.chars() {
        if c == '\'' && p != '\\' && !d { s = !s; }
        if c == '"' && p != '\\' && !s { d = !d; }
        p = c;
    }
    s || d
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_space_before_semicolon() {
        let edits = NoSinglelineWhitespaceBeforeSemicolonsFixer.check("<?php\n$a = 1 ;", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, ";");
    }

    #[test]
    fn test_no_space() {
        let edits = NoSinglelineWhitespaceBeforeSemicolonsFixer.check("<?php\n$a = 1;", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
