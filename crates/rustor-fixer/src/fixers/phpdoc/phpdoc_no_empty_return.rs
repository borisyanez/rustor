//! Remove @return void when obvious

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocNoEmptyReturnFixer;

impl Fixer for PhpdocNoEmptyReturnFixer {
    fn name(&self) -> &'static str { "phpdoc_no_empty_return" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_no_empty_return" }
    fn description(&self) -> &'static str { "Remove @return void" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match @return void in PHPDoc
        let re = Regex::new(r"\n\s*\*\s*@return\s+void\s*\n").unwrap();

        for m in re.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(),
                "\n".to_string(),
                "Remove @return void".to_string(),
                "phpdoc_no_empty_return",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_no_return_void() {
        let code = "<?php
/**
 * @return void
 */";
        let edits = PhpdocNoEmptyReturnFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_return_int_unchanged() {
        let code = "<?php
/**
 * @return int
 */";
        let edits = PhpdocNoEmptyReturnFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
