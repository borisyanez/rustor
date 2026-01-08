//! Standardize increment style

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct StandardizeIncrementFixer;

impl Fixer for StandardizeIncrementFixer {
    fn name(&self) -> &'static str { "standardize_increment" }
    fn php_cs_fixer_name(&self) -> &'static str { "standardize_increment" }
    fn description(&self) -> &'static str { "Use pre-increment/decrement" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match $var++ or $var-- as standalone statements (followed by ; or in for loop)
        // Pre-increment is more efficient in PHP for objects

        // Post-increment as statement: $i++;
        let re = Regex::new(r"(\$\w+)\+\+(\s*;)").unwrap();
        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let var = cap.get(1).unwrap().as_str();
            let semi = cap.get(2).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("++{}{}", var, semi),
                "Use pre-increment".to_string(),
                "standardize_increment",
            ));
        }

        // Post-decrement as statement: $i--;
        let re = Regex::new(r"(\$\w+)--(\s*;)").unwrap();
        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let var = cap.get(1).unwrap().as_str();
            let semi = cap.get(2).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("--{}{}", var, semi),
                "Use pre-decrement".to_string(),
                "standardize_increment",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_post_increment() {
        let code = "<?php\n$i++;";
        let edits = StandardizeIncrementFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_post_decrement() {
        let code = "<?php\n$i--;";
        let edits = StandardizeIncrementFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_pre_increment() {
        let code = "<?php\n++$i;";
        let edits = StandardizeIncrementFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
