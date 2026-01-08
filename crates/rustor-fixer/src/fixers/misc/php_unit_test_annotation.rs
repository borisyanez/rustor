//! PHPUnit test annotation

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpUnitTestAnnotationFixer;

impl Fixer for PhpUnitTestAnnotationFixer {
    fn name(&self) -> &'static str { "php_unit_test_annotation" }
    fn php_cs_fixer_name(&self) -> &'static str { "php_unit_test_annotation" }
    fn description(&self) -> &'static str { "Use test prefix or @test" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Find methods with @test annotation that don't have test prefix
        // Convert to test prefix (default style)
        let re = Regex::new(r"(?ms)/\*\*[^*]*\*\s*@test\s*\n[^*]*\*/\s*(?:public\s+)?function\s+(\w+)\s*\(").unwrap();

        for cap in re.captures_iter(source) {
            let method = cap.get(1).unwrap().as_str();

            // If method doesn't start with test, suggest adding it
            if !method.starts_with("test") {
                edits.push(edit_with_rule(
                    cap.get(1).unwrap().start(), cap.get(1).unwrap().end(),
                    format!("test{}{}", method.chars().next().unwrap().to_ascii_uppercase(), &method[1..]),
                    "Use test prefix instead of @test annotation".to_string(),
                    "php_unit_test_annotation",
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
    fn test_annotation_without_prefix() {
        let code = "<?php\n/**\n * @test\n */\nfunction someBehavior() {}";
        let edits = PhpUnitTestAnnotationFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_prefix_no_annotation() {
        let code = "<?php\nfunction testSomeBehavior() {}";
        let edits = PhpUnitTestAnnotationFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
