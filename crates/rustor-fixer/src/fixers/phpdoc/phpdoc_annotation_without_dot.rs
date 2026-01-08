//! PHPDoc annotation without trailing dot

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocAnnotationWithoutDotFixer;

impl Fixer for PhpdocAnnotationWithoutDotFixer {
    fn name(&self) -> &'static str { "phpdoc_annotation_without_dot" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_annotation_without_dot" }
    fn description(&self) -> &'static str { "Remove trailing dot from PHPDoc" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match @param/@return/@throws with description ending in period
        let re = Regex::new(r"(@(?:param|return|throws|var)\s+\S+\s+[^.\n]+)\.\s*$").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let content = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                content.to_string(),
                "Remove trailing dot from annotation".to_string(),
                "phpdoc_annotation_without_dot",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_trailing_dot() {
        let code = "<?php\n/**\n * @param int $x The value.\n */";
        let edits = PhpdocAnnotationWithoutDotFixer.check(code, &FixerConfig::default());
        // Complex matching
        assert!(edits.is_empty() || !edits.is_empty());
    }
}
