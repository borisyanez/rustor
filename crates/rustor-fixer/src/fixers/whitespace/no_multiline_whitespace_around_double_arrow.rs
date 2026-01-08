//! No multiline whitespace around double arrow

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoMultilineWhitespaceAroundDoubleArrowFixer;

impl Fixer for NoMultilineWhitespaceAroundDoubleArrowFixer {
    fn name(&self) -> &'static str { "no_multiline_whitespace_around_double_arrow" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_multiline_whitespace_around_double_arrow" }
    fn description(&self) -> &'static str { "No multiline around =>" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match => with newline before or after
        let re = Regex::new(r"(\S)\s*\n\s*(=>)").unwrap();
        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let before = cap.get(1).unwrap().as_str();
            let arrow = cap.get(2).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{} {}", before, arrow),
                "Remove newline before =>".to_string(),
                "no_multiline_whitespace_around_double_arrow",
            ));
        }

        let re2 = Regex::new(r"(=>)\s*\n\s*(\S)").unwrap();
        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let arrow = cap.get(1).unwrap().as_str();
            let after = cap.get(2).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{} {}", arrow, after),
                "Remove newline after =>".to_string(),
                "no_multiline_whitespace_around_double_arrow",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_newline_before_arrow() {
        let code = "<?php\n$a = ['key'\n=> 'value'];";
        let edits = NoMultilineWhitespaceAroundDoubleArrowFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
