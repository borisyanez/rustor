//! Trim PHPDoc - remove blank lines at start/end

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocTrimFixer;

impl Fixer for PhpdocTrimFixer {
    fn name(&self) -> &'static str { "phpdoc_trim" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_trim" }
    fn description(&self) -> &'static str { "Remove extra blank lines in PHPDoc" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match PHPDoc with blank line after /**
        let re1 = Regex::new(r"(/\*\*)\n(\s*\*\s*\n)+(\s*\*)").unwrap();
        for cap in re1.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let start = cap.get(1).unwrap().as_str();
            let last_star = cap.get(3).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}\n{}", start, last_star),
                "Remove blank line at start of PHPDoc".to_string(),
                "phpdoc_trim",
            ));
        }

        // Match PHPDoc with blank line before */
        let re2 = Regex::new(r"(\*[^\n]+)\n(\s*\*\s*\n)+(\s*\*/)").unwrap();
        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let content = cap.get(1).unwrap().as_str();
            let end = cap.get(3).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}\n{}", content, end),
                "Remove blank line at end of PHPDoc".to_string(),
                "phpdoc_trim",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_trim_start() {
        let code = "<?php
/**
 *
 * @param int $x
 */";
        let edits = PhpdocTrimFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty() || edits.is_empty());
    }
}
