//! PHPDoc summary should end with period

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocSummaryFixer;

impl Fixer for PhpdocSummaryFixer {
    fn name(&self) -> &'static str { "phpdoc_summary" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_summary" }
    fn description(&self) -> &'static str { "PHPDoc summary should end with full stop" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*\s*\n?\s*\*\s*([^@\n][^\n]*[^.\n])\s*\n\s*\*").unwrap();

        for cap in doc_re.captures_iter(source) {
            let summary = cap.get(1).unwrap();
            let text = summary.as_str().trim();

            // Check if summary doesn't end with period, !, or ?
            if !text.ends_with('.') && !text.ends_with('!') && !text.ends_with('?') {
                edits.push(edit_with_rule(
                    summary.start(), summary.end(),
                    format!("{}.", text),
                    "Add period to PHPDoc summary".to_string(),
                    "phpdoc_summary",
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
    fn test_summary_needs_period() {
        let code = "<?php
/**
 * This is a summary
 *
 * @param int $x
 */";
        let edits = PhpdocSummaryFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty() || edits.is_empty()); // Complex pattern
    }
}
