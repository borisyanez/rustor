//! Lowercase hex and binary literals

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct IntegerLiteralCaseFixer;

impl Fixer for IntegerLiteralCaseFixer {
    fn name(&self) -> &'static str { "integer_literal_case" }
    fn php_cs_fixer_name(&self) -> &'static str { "integer_literal_case" }
    fn description(&self) -> &'static str { "Lowercase hex and binary literals" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match hex with uppercase: 0X or uppercase letters A-F
        let hex_re = Regex::new(r"\b(0[xX])([0-9a-fA-F]+)\b").unwrap();

        for cap in hex_re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let prefix = cap.get(1).unwrap().as_str();
            let digits = cap.get(2).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            let lower = format!("0x{}", digits.to_lowercase());
            if full.as_str() != lower {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    lower,
                    "Use lowercase hex".to_string(),
                    "integer_literal_case",
                ));
            }
        }

        // Match binary with uppercase: 0B
        let bin_re = Regex::new(r"\b0B([01]+)\b").unwrap();

        for cap in bin_re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let digits = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("0b{}", digits),
                "Use lowercase binary".to_string(),
                "integer_literal_case",
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
    fn test_uppercase_hex() {
        let edits = IntegerLiteralCaseFixer.check("<?php\n$a = 0xFF;", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_lowercase_hex() {
        let edits = IntegerLiteralCaseFixer.check("<?php\n$a = 0xff;", &FixerConfig::default());
        assert!(edits.is_empty());
    }

    #[test]
    fn test_uppercase_binary() {
        let edits = IntegerLiteralCaseFixer.check("<?php\n$a = 0B1010;", &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
