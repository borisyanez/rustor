//! Combine consecutive unset calls

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct CombineConsecutiveUnsetsFixer;

impl Fixer for CombineConsecutiveUnsetsFixer {
    fn name(&self) -> &'static str { "combine_consecutive_unsets" }
    fn php_cs_fixer_name(&self) -> &'static str { "combine_consecutive_unsets" }
    fn description(&self) -> &'static str { "Combine consecutive unset() calls" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: unset($a); unset($b);
        let re = Regex::new(r"unset\s*\(([^)]+)\)\s*;\s*\n?\s*unset\s*\(([^)]+)\)\s*;").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let arg1 = cap.get(1).unwrap().as_str().trim();
            let arg2 = cap.get(2).unwrap().as_str().trim();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("unset({}, {});", arg1, arg2),
                "Combine unset() calls".to_string(),
                "combine_consecutive_unsets",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_combine_unsets() {
        let edits = CombineConsecutiveUnsetsFixer.check("<?php\nunset($a);\nunset($b);", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("unset($a, $b)"));
    }
}
