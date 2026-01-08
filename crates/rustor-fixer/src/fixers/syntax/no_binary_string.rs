//! Remove b prefix from strings

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoBinaryStringFixer;

impl Fixer for NoBinaryStringFixer {
    fn name(&self) -> &'static str { "no_binary_string" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_binary_string" }
    fn description(&self) -> &'static str { "Remove b prefix from strings" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match b"string" or b'string'
        let re = Regex::new(r#"\bb(['"])"#).unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let quote = cap.get(1).unwrap().as_str();

            // Check not in string context
            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                quote.to_string(),
                "Remove b prefix from string".to_string(),
                "no_binary_string",
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
    fn test_binary_string() {
        let edits = NoBinaryStringFixer.check("<?php\n$a = b'hello';", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_normal_string() {
        let edits = NoBinaryStringFixer.check("<?php\n$a = 'hello';", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
