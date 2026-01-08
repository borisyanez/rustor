//! Order PHPDoc types alphabetically

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocTypesOrderFixer;

impl Fixer for PhpdocTypesOrderFixer {
    fn name(&self) -> &'static str { "phpdoc_types_order" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_types_order" }
    fn description(&self) -> &'static str { "Order PHPDoc types" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*.*?\*/").unwrap();

        for doc_match in doc_re.find_iter(source) {
            let doc = doc_match.as_str();
            let start = doc_match.start();

            // Match union types: @param int|string|null $var
            let re = Regex::new(r"(@(?:param|return|var|throws)\s+)([\w\\|]+)").unwrap();

            for cap in re.captures_iter(doc) {
                let full = cap.get(0).unwrap();
                let prefix = cap.get(1).unwrap().as_str();
                let types = cap.get(2).unwrap().as_str();

                if types.contains('|') {
                    let mut parts: Vec<&str> = types.split('|').collect();
                    let original = parts.clone();

                    // Sort with null last
                    parts.sort_by(|a, b| {
                        if *a == "null" { std::cmp::Ordering::Greater }
                        else if *b == "null" { std::cmp::Ordering::Less }
                        else { a.to_lowercase().cmp(&b.to_lowercase()) }
                    });

                    if parts != original {
                        edits.push(edit_with_rule(
                            start + full.start(), start + full.end(),
                            format!("{}{}", prefix, parts.join("|")),
                            "Order PHPDoc types".to_string(),
                            "phpdoc_types_order",
                        ));
                    }
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
    fn test_types_order() {
        let code = "<?php
/**
 * @param null|string|int $x
 */";
        let edits = PhpdocTypesOrderFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("int|string|null"));
    }
}
