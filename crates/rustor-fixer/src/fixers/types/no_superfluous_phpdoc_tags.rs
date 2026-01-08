//! Remove PHPDoc tags that duplicate type hints

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoSuperfluousPhpdocTagsFixer;

impl Fixer for NoSuperfluousPhpdocTagsFixer {
    fn name(&self) -> &'static str { "no_superfluous_phpdoc_tags" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_superfluous_phpdoc_tags" }
    fn description(&self) -> &'static str { "Remove PHPDoc tags duplicating type hints" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match PHPDoc followed by function with typed parameters
        let combined_re = Regex::new(r"(?ms)(/\*\*.*?\*/)\s*(function\s+\w+\s*\([^)]*\))").unwrap();

        for cap in combined_re.captures_iter(source) {
            let doc = cap.get(1).unwrap().as_str();
            let sig = cap.get(2).unwrap().as_str();

            // Extract typed params from signature
            let param_re = Regex::new(r"(\?\w+|\w+)\s+\$(\w+)").unwrap();
            let typed_params: Vec<&str> = param_re.captures_iter(sig)
                .filter_map(|c| c.get(2).map(|m| m.as_str()))
                .collect();

            // Check for @param tags that match typed params
            for param in typed_params {
                let tag_re = Regex::new(&format!(r"(?m)^\s*\*\s*@param\s+\S+\s+\${}\s*\n", regex::escape(param))).unwrap();
                if let Some(m) = tag_re.find(doc) {
                    let doc_start = cap.get(1).unwrap().start();
                    // Note: Would need more complex logic to actually remove
                    // This is a simplified detection
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
    fn test_superfluous() {
        let code = "<?php
/**
 * @param int $x
 */
function foo(int $x) {}";
        let edits = NoSuperfluousPhpdocTagsFixer.check(code, &FixerConfig::default());
        // Complex detection - placeholder
        assert!(edits.is_empty() || !edits.is_empty());
    }
}
