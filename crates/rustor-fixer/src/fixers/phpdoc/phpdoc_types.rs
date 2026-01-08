//! Normalize PHPDoc types to lowercase

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocTypesFixer;

impl Fixer for PhpdocTypesFixer {
    fn name(&self) -> &'static str { "phpdoc_types" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_types" }
    fn description(&self) -> &'static str { "Normalize PHPDoc types" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let doc_re = Regex::new(r"(?ms)/\*\*.*?\*/").unwrap();

        // Types that should be lowercase in PHPDoc
        let types = [
            ("Boolean", "bool"), ("Integer", "int"), ("Double", "float"),
            ("Real", "float"), ("NULL", "null"), ("TRUE", "true"),
            ("FALSE", "false"), ("VOID", "void"), ("MIXED", "mixed"),
        ];

        for doc_match in doc_re.find_iter(source) {
            let doc = doc_match.as_str();
            let start = doc_match.start();

            for (from, to) in &types {
                let re = Regex::new(&format!(r"(@(?:param|return|var|throws)\s+)(?:\w+\|)*{}\b", from)).unwrap();

                for cap in re.captures_iter(doc) {
                    let full = cap.get(0).unwrap();
                    let new_text = full.as_str().replace(from, to);

                    edits.push(edit_with_rule(
                        start + full.start(), start + full.end(),
                        new_text,
                        format!("Use {} instead of {}", to, from),
                        "phpdoc_types",
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
    fn test_boolean_to_bool() {
        let code = "<?php\n/**\n * @param Boolean $x\n */";
        let edits = PhpdocTypesFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
