//! PHPDoc inline tag normalizer

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocInlineTagNormalizerFixer;

impl Fixer for PhpdocInlineTagNormalizerFixer {
    fn name(&self) -> &'static str { "phpdoc_inline_tag_normalizer" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_inline_tag_normalizer" }
    fn description(&self) -> &'static str { "Normalize inline PHPDoc tags" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Normalize {@inheritdoc} spacing
        let re = Regex::new(r"\{\s*@(\w+)\s*\}").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let tag = cap.get(1).unwrap().as_str();

            let normalized = format!("{{@{}}}", tag);
            if full.as_str() != normalized {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    normalized,
                    "Normalize inline tag spacing".to_string(),
                    "phpdoc_inline_tag_normalizer",
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
    fn test_inline_tag_spacing() {
        let code = "<?php\n/**\n * {  @inheritdoc  }\n */";
        let edits = PhpdocInlineTagNormalizerFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_normalized_inline_tag() {
        let code = "<?php\n/**\n * {@inheritdoc}\n */";
        let edits = PhpdocInlineTagNormalizerFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
