//! PHPDoc @var annotation correct order

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocVarAnnotationCorrectOrderFixer;

impl Fixer for PhpdocVarAnnotationCorrectOrderFixer {
    fn name(&self) -> &'static str { "phpdoc_var_annotation_correct_order" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_var_annotation_correct_order" }
    fn description(&self) -> &'static str { "@var type $var order" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match @var $var type (wrong order) - should be @var type $var
        let re = Regex::new(r"(@var)\s+(\$\w+)\s+(\S+)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let tag = cap.get(1).unwrap().as_str();
            let var = cap.get(2).unwrap().as_str();
            let type_hint = cap.get(3).unwrap().as_str();

            // Check if type_hint looks like a type (not starting with $)
            if !type_hint.starts_with('$') {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{} {} {}", tag, type_hint, var),
                    "Correct @var order: type before variable".to_string(),
                    "phpdoc_var_annotation_correct_order",
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
    fn test_wrong_order() {
        let code = "<?php\n/** @var $x int */";
        let edits = PhpdocVarAnnotationCorrectOrderFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_correct_order() {
        let code = "<?php\n/** @var int $x */";
        let edits = PhpdocVarAnnotationCorrectOrderFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
