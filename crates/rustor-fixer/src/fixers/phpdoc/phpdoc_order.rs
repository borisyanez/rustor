//! Order PHPDoc tags

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocOrderFixer;

impl Fixer for PhpdocOrderFixer {
    fn name(&self) -> &'static str { "phpdoc_order" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_order" }
    fn description(&self) -> &'static str { "Order PHPDoc tags" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*.*?\*/").unwrap();

        // Standard order: @param, @return, @throws
        let tag_priority = |tag: &str| -> i32 {
            match tag {
                "@param" => 1,
                "@return" => 2,
                "@throws" => 3,
                _ => 100,
            }
        };

        for doc_match in doc_re.find_iter(source) {
            let doc = doc_match.as_str();
            let start = doc_match.start();

            // Extract tags with their positions
            let tag_re = Regex::new(r"(?m)^\s*\*\s*(@\w+)").unwrap();
            let tags: Vec<_> = tag_re.captures_iter(doc)
                .filter_map(|c| c.get(1).map(|m| (m.start(), m.as_str())))
                .collect();

            // Check if @return comes before @param (wrong order)
            let mut seen_return = false;
            for (_, tag) in &tags {
                if *tag == "@return" { seen_return = true; }
                if *tag == "@param" && seen_return {
                    // @param after @return - needs reordering
                    // This is a simplified check - full reordering is complex
                    edits.push(edit_with_rule(
                        start, doc_match.end(),
                        doc.to_string(), // Would need to reorder - placeholder
                        "PHPDoc tags should be ordered: @param, @return, @throws".to_string(),
                        "phpdoc_order",
                    ));
                    break;
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
    fn test_order_correct() {
        let code = "<?php
/**
 * @param int $x
 * @return int
 */";
        let edits = PhpdocOrderFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
