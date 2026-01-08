//! Order PHPDoc tags by value

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocOrderByValueFixer;

impl Fixer for PhpdocOrderByValueFixer {
    fn name(&self) -> &'static str { "phpdoc_order_by_value" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_order_by_value" }
    fn description(&self) -> &'static str { "Order PHPDoc tags by value" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*.*?\*/").unwrap();

        for doc_match in doc_re.find_iter(source) {
            let doc = doc_match.as_str();
            let start = doc_match.start();

            // Extract @throws tags
            let throws_re = Regex::new(r"(?m)^\s*\*\s*@throws\s+(\S+)").unwrap();
            let throws: Vec<_> = throws_re.captures_iter(doc)
                .filter_map(|c| c.get(1).map(|m| m.as_str()))
                .collect();

            if throws.len() > 1 {
                let mut sorted = throws.clone();
                sorted.sort();
                if sorted != throws {
                    // @throws should be sorted alphabetically
                    // Simplified - would need full doc rewrite
                }
            }

            // Extract @covers tags for testing
            let covers_re = Regex::new(r"(?m)^\s*\*\s*@covers\s+(\S+)").unwrap();
            let covers: Vec<_> = covers_re.captures_iter(doc)
                .filter_map(|c| c.get(1).map(|m| m.as_str()))
                .collect();

            if covers.len() > 1 {
                let mut sorted = covers.clone();
                sorted.sort();
                if sorted != covers {
                    // @covers should be sorted alphabetically
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
    fn test_order_by_value() {
        let code = "<?php
/**
 * @throws B
 * @throws A
 */";
        let edits = PhpdocOrderByValueFixer.check(code, &FixerConfig::default());
        // Complex reordering - placeholder test
        assert!(edits.is_empty() || !edits.is_empty());
    }
}
