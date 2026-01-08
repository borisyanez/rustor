//! No whitespace before comma in array

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoWhitespaceBeforeCommaInArrayFixer;

impl Fixer for NoWhitespaceBeforeCommaInArrayFixer {
    fn name(&self) -> &'static str { "no_whitespace_before_comma_in_array" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_whitespace_before_comma_in_array" }
    fn description(&self) -> &'static str { "No space before comma in array" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match space before comma
        let re = Regex::new(r"(\S)[ \t]+,").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let before = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{},", before),
                "Remove space before comma".to_string(),
                "no_whitespace_before_comma_in_array",
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
    fn test_space_before_comma() {
        let edits = NoWhitespaceBeforeCommaInArrayFixer.check("<?php\n$a = [1 , 2];", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_no_space() {
        let edits = NoWhitespaceBeforeCommaInArrayFixer.check("<?php\n$a = [1, 2];", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
