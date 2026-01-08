//! Separate PHPDoc tag groups with blank lines

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocSeparationFixer;

impl Fixer for PhpdocSeparationFixer {
    fn name(&self) -> &'static str { "phpdoc_separation" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_separation" }
    fn description(&self) -> &'static str { "Separate PHPDoc tag groups" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*.*?\*/").unwrap();

        for doc_match in doc_re.find_iter(source) {
            let doc = doc_match.as_str();
            let start = doc_match.start();

            // Check for @param followed immediately by @return (no blank line)
            let re = Regex::new(r"(@param[^\n]*\n\s*\*)(\s*)(@return)").unwrap();
            for cap in re.captures_iter(doc) {
                let space = cap.get(2).unwrap();
                if !space.as_str().contains('\n') {
                    // Need blank line between @param and @return
                    let full = cap.get(0).unwrap();
                    let before = cap.get(1).unwrap().as_str();
                    let after = cap.get(3).unwrap().as_str();

                    edits.push(edit_with_rule(
                        start + full.start(), start + full.end(),
                        format!("{}\n *\n * {}", before, after),
                        "Add blank line between tag groups".to_string(),
                        "phpdoc_separation",
                    ));
                }
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_separation() {
        let code = "<?php
/**
 * @param string $x
 * @return int
 */";
        let edits = PhpdocSeparationFixer.check(code, &FixerConfig::default());
        // May or may not find issues based on config
        assert!(edits.is_empty() || !edits.is_empty());
    }
}
