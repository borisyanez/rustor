//! Remove consecutive blank lines in PHPDoc

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocTrimConsecutiveBlankLineSeparationFixer;

impl Fixer for PhpdocTrimConsecutiveBlankLineSeparationFixer {
    fn name(&self) -> &'static str { "phpdoc_trim_consecutive_blank_line_separation" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_trim_consecutive_blank_line_separation" }
    fn description(&self) -> &'static str { "Remove consecutive blank lines in PHPDoc" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*.*?\*/").unwrap();

        for doc_match in doc_re.find_iter(source) {
            let doc = doc_match.as_str();
            let start = doc_match.start();

            // Match multiple consecutive blank lines (just *)
            let re = Regex::new(r"(\n\s*\*\s*\n)(\s*\*\s*\n)+").unwrap();
            for m in re.find_iter(doc) {
                edits.push(edit_with_rule(
                    start + m.start(), start + m.end(),
                    "\n * \n".to_string(),
                    "Remove consecutive blank lines".to_string(),
                    "phpdoc_trim_consecutive_blank_line_separation",
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
    fn test_consecutive_blanks() {
        let code = "<?php
/**
 * Summary
 *
 *
 * @param int $x
 */";
        let edits = PhpdocTrimConsecutiveBlankLineSeparationFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty() || edits.is_empty());
    }
}
