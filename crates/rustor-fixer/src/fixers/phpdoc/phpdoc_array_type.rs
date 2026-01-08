//! Normalize PHPDoc array type notation

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocArrayTypeFixer;

impl Fixer for PhpdocArrayTypeFixer {
    fn name(&self) -> &'static str { "phpdoc_array_type" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_array_type" }
    fn description(&self) -> &'static str { "Normalize PHPDoc array type" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*.*?\*/").unwrap();

        for doc_match in doc_re.find_iter(source) {
            let doc = doc_match.as_str();
            let start = doc_match.start();

            // Convert array<Type> to Type[] (short form)
            let re = Regex::new(r"array<(\w+)>").unwrap();
            for cap in re.captures_iter(doc) {
                let full = cap.get(0).unwrap();
                let inner = cap.get(1).unwrap().as_str();

                edits.push(edit_with_rule(
                    start + full.start(), start + full.end(),
                    format!("{}[]", inner),
                    "Use short array notation".to_string(),
                    "phpdoc_array_type",
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
    fn test_array_type() {
        let code = "<?php
/**
 * @param array<string> $x
 */";
        let edits = PhpdocArrayTypeFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("string[]"));
    }
}
