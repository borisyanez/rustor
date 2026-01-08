//! Combine consecutive isset calls

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct CombineConsecutiveIssetsFixer;

impl Fixer for CombineConsecutiveIssetsFixer {
    fn name(&self) -> &'static str { "combine_consecutive_issets" }
    fn php_cs_fixer_name(&self) -> &'static str { "combine_consecutive_issets" }
    fn description(&self) -> &'static str { "Combine consecutive isset() calls" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: isset($a) && isset($b)
        let re = Regex::new(r"isset\s*\(([^)]+)\)\s*&&\s*isset\s*\(([^)]+)\)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let arg1 = cap.get(1).unwrap().as_str().trim();
            let arg2 = cap.get(2).unwrap().as_str().trim();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("isset({}, {})", arg1, arg2),
                "Combine isset() calls".to_string(),
                "combine_consecutive_issets",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_combine_issets() {
        let edits = CombineConsecutiveIssetsFixer.check("<?php\nif (isset($a) && isset($b)) {}", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("isset($a, $b)"));
    }
}
