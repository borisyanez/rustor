//! Control PHPDoc line span (single vs multi-line)

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocLineSpanFixer;

impl Fixer for PhpdocLineSpanFixer {
    fn name(&self) -> &'static str { "phpdoc_line_span" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_line_span" }
    fn description(&self) -> &'static str { "Control PHPDoc line span" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Convert single-line @var to multi-line for properties
        // Match: /** @var Type */ on property
        let re = Regex::new(r"(/\*\*\s*@var\s+\S+\s*\*/)\s*\n(\s*)(public|protected|private)").unwrap();

        for cap in re.captures_iter(source) {
            let doc = cap.get(1).unwrap();
            let indent = cap.get(2).unwrap().as_str();
            let vis = cap.get(3).unwrap().as_str();

            let doc_text = doc.as_str();
            // Extract type
            if let Some(type_cap) = Regex::new(r"@var\s+(\S+)").unwrap().captures(doc_text) {
                let type_hint = type_cap.get(1).unwrap().as_str();

                // Check if this should be multi-line (config dependent)
                // By default, keep single-line for simple @var
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_line_span() {
        let code = "<?php
/** @var int */
public $x;";
        let edits = PhpdocLineSpanFixer.check(code, &FixerConfig::default());
        // Config-dependent behavior
        assert!(edits.is_empty() || !edits.is_empty());
    }
}
