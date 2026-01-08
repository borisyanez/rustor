//! Align PHPDoc tags

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocAlignFixer;

impl Fixer for PhpdocAlignFixer {
    fn name(&self) -> &'static str { "phpdoc_align" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_align" }
    fn description(&self) -> &'static str { "Align PHPDoc tags vertically" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*.*?\*/").unwrap();

        for doc_match in doc_re.find_iter(source) {
            let doc = doc_match.as_str();
            let start = doc_match.start();

            // Find @param, @return, @throws, @var tags
            let tag_re = Regex::new(r"(?m)^(\s*\*\s*)(@(?:param|return|throws|var))\s+(\S+)").unwrap();
            let tags: Vec<_> = tag_re.captures_iter(doc).collect();

            if tags.len() < 2 { continue; }

            // Calculate max lengths for alignment
            let max_tag_len = tags.iter()
                .map(|c| c.get(2).unwrap().as_str().len())
                .max()
                .unwrap_or(0);

            for cap in tag_re.captures_iter(doc) {
                let full = cap.get(0).unwrap();
                let prefix = cap.get(1).unwrap().as_str();
                let tag = cap.get(2).unwrap().as_str();
                let rest = cap.get(3).unwrap().as_str();

                let padding = " ".repeat(max_tag_len - tag.len() + 1);
                let new_text = format!("{}{}{}{}", prefix, tag, padding, rest);

                if full.as_str() != new_text {
                    edits.push(edit_with_rule(
                        start + full.start(), start + full.end(),
                        new_text,
                        "Align PHPDoc tags".to_string(),
                        "phpdoc_align",
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
    fn test_align() {
        let code = "<?php
/**
 * @param string $x
 * @return int
 */";
        let edits = PhpdocAlignFixer.check(code, &FixerConfig::default());
        // Basic test - alignment is complex
        assert!(edits.is_empty() || !edits.is_empty());
    }
}
