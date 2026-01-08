//! Clean namespace - remove leading/trailing whitespace

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct CleanNamespaceFixer;

impl Fixer for CleanNamespaceFixer {
    fn name(&self) -> &'static str { "clean_namespace" }
    fn php_cs_fixer_name(&self) -> &'static str { "clean_namespace" }
    fn description(&self) -> &'static str { "Clean namespace declarations" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Remove leading backslash from namespace
        let re = Regex::new(r"\bnamespace\s+\\([A-Za-z])").unwrap();
        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let first_char = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("namespace {}", first_char),
                "Remove leading backslash from namespace".to_string(),
                "clean_namespace",
            ));
        }

        // Remove extra spaces in namespace
        let re2 = Regex::new(r"\bnamespace\s{2,}(\S)").unwrap();
        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let after = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("namespace {}", after),
                "Single space after namespace".to_string(),
                "clean_namespace",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_leading_backslash() {
        let edits = CleanNamespaceFixer.check("<?php\nnamespace \\App;", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_clean_namespace() {
        let edits = CleanNamespaceFixer.check("<?php\nnamespace App;", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
