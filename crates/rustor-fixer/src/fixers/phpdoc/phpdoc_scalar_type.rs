//! Use scalar types in PHPDoc

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocScalarTypeFixer;

impl Fixer for PhpdocScalarTypeFixer {
    fn name(&self) -> &'static str { "phpdoc_scalar" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_scalar" }
    fn description(&self) -> &'static str { "Use scalar type hints in PHPDoc" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*.*?\*/").unwrap();

        // Map of aliases to canonical types
        let type_map = [
            ("integer", "int"),
            ("boolean", "bool"),
            ("real", "float"),
            ("double", "float"),
        ];

        for doc_match in doc_re.find_iter(source) {
            let doc = doc_match.as_str();
            let start = doc_match.start();

            for (from, to) in &type_map {
                let re = Regex::new(&format!(r"(@(?:param|return|var|throws)\s+)\b{}\b", from)).unwrap();
                for cap in re.captures_iter(doc) {
                    let full = cap.get(0).unwrap();
                    let prefix = cap.get(1).unwrap().as_str();

                    edits.push(edit_with_rule(
                        start + full.start(), start + full.end(),
                        format!("{}{}", prefix, to),
                        format!("Use {} instead of {}", to, from),
                        "phpdoc_scalar",
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
    fn test_integer_to_int() {
        let code = "<?php
/**
 * @param integer $x
 */";
        let edits = PhpdocScalarTypeFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("int"));
    }

    #[test]
    fn test_boolean_to_bool() {
        let code = "<?php
/**
 * @return boolean
 */";
        let edits = PhpdocScalarTypeFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
