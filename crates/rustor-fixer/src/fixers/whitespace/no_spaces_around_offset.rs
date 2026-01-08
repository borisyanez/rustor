//! No spaces around array offset brackets

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoSpacesAroundOffsetFixer;

impl Fixer for NoSpacesAroundOffsetFixer {
    fn name(&self) -> &'static str { "no_spaces_around_offset" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_spaces_around_offset" }
    fn description(&self) -> &'static str { "No spaces in array access" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match $var[ space or space ]
        let re1 = Regex::new(r"(\$\w+)\[\s+").unwrap();
        for cap in re1.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let var = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}[", var),
                "Remove space after [".to_string(),
                "no_spaces_around_offset",
            ));
        }

        let re2 = Regex::new(r"\s+\]").unwrap();
        for m in re2.find_iter(source) {
            // Check if this is array access context
            let before = &source[..m.start()];
            if before.contains('[') && !is_in_string(before) {
                edits.push(edit_with_rule(
                    m.start(), m.end(),
                    "]".to_string(),
                    "Remove space before ]".to_string(),
                    "no_spaces_around_offset",
                ));
            }
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
    fn test_space_after_bracket() {
        let edits = NoSpacesAroundOffsetFixer.check("<?php\n$a[ 0];", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_space_before_bracket() {
        let edits = NoSpacesAroundOffsetFixer.check("<?php\n$a[0 ];", &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
