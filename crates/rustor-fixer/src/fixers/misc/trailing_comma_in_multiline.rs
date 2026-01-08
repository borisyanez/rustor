//! Add trailing comma in multiline structures

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct TrailingCommaInMultilineFixer;

impl Fixer for TrailingCommaInMultilineFixer {
    fn name(&self) -> &'static str { "trailing_comma_in_multiline" }
    fn php_cs_fixer_name(&self) -> &'static str { "trailing_comma_in_multiline" }
    fn description(&self) -> &'static str { "Add trailing comma in multiline" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match multiline array without trailing comma
        // Pattern: value followed by newline and closing bracket
        let re = Regex::new(r"([^\s,\[\{])\s*\n(\s*)\]").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let last_char = cap.get(1).unwrap().as_str();
            let indent = cap.get(2).unwrap().as_str();

            // Skip if already has comma or is opening bracket
            if last_char != "[" && last_char != "{" {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{},\n{}]", last_char, indent),
                    "Add trailing comma".to_string(),
                    "trailing_comma_in_multiline",
                ));
            }
        }

        // Same for function parameters
        let re2 = Regex::new(r"([^\s,\(])\s*\n(\s*)\)").unwrap();
        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let last_char = cap.get(1).unwrap().as_str();
            let indent = cap.get(2).unwrap().as_str();

            if last_char != "(" {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{},\n{})", last_char, indent),
                    "Add trailing comma".to_string(),
                    "trailing_comma_in_multiline",
                ));
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_trailing_comma_array() {
        let code = "<?php
$a = [
    1,
    2
];";
        let edits = TrailingCommaInMultilineFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
