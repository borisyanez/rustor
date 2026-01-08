//! Fix PHPDoc tag format (inline vs regular)

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocTagTypeFixer;

impl Fixer for PhpdocTagTypeFixer {
    fn name(&self) -> &'static str { "phpdoc_tag_type" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_tag_type" }
    fn description(&self) -> &'static str { "Fix PHPDoc tag type format" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Convert {@inheritdoc} to @inheritDoc (annotation style)
        let re = Regex::new(r"\{@inheritdoc\}").unwrap();
        for m in re.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(),
                "@inheritDoc".to_string(),
                "Use @inheritDoc annotation style".to_string(),
                "phpdoc_tag_type",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_inheritdoc_format() {
        let code = "<?php
/**
 * {@inheritdoc}
 */";
        let edits = PhpdocTagTypeFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("@inheritDoc"));
    }
}
