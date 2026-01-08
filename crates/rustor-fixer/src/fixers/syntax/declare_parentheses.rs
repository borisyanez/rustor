//! No space in declare()

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct DeclareParenthesesFixer;

impl Fixer for DeclareParenthesesFixer {
    fn name(&self) -> &'static str { "declare_parentheses" }
    fn php_cs_fixer_name(&self) -> &'static str { "declare_parentheses" }
    fn description(&self) -> &'static str { "No space in declare()" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match declare with spaces: declare( strict_types=1 )
        let re = Regex::new(r"\bdeclare\s*\(\s*(\w+)\s*=\s*(\w+)\s*\)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let directive = cap.get(1).unwrap().as_str();
            let value = cap.get(2).unwrap().as_str();

            let clean = format!("declare({}={})", directive, value);
            if full.as_str() != clean {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    clean,
                    "Remove spaces in declare()".to_string(),
                    "declare_parentheses",
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
    fn test_spaces_in_declare() {
        let edits = DeclareParenthesesFixer.check("<?php\ndeclare( strict_types = 1 );", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_clean_declare() {
        let edits = DeclareParenthesesFixer.check("<?php\ndeclare(strict_types=1);", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
