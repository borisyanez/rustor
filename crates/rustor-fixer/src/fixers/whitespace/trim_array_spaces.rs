//! Remove spaces inside array brackets

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct TrimArraySpacesFixer;

impl Fixer for TrimArraySpacesFixer {
    fn name(&self) -> &'static str { "trim_array_spaces" }
    fn php_cs_fixer_name(&self) -> &'static str { "trim_array_spaces" }
    fn description(&self) -> &'static str { "No spaces inside array brackets" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match [ space at start of array
        let re1 = Regex::new(r"\[\s+([^\]\s])").unwrap();
        for cap in re1.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let first = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("[{}", first),
                "Remove space after [".to_string(),
                "trim_array_spaces",
            ));
        }

        // Match space ] at end of array
        let re2 = Regex::new(r"([^\[\s])\s+\]").unwrap();
        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let last = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}]", last),
                "Remove space before ]".to_string(),
                "trim_array_spaces",
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
    fn test_space_after_open() {
        let edits = TrimArraySpacesFixer.check("<?php\n$a = [ 1, 2 ];", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_no_spaces() {
        let edits = TrimArraySpacesFixer.check("<?php\n$a = [1, 2];", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
