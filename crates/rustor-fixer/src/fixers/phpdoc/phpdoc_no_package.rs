//! Remove @package and @subpackage tags

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocNoPackageFixer;

impl Fixer for PhpdocNoPackageFixer {
    fn name(&self) -> &'static str { "phpdoc_no_package" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_no_package" }
    fn description(&self) -> &'static str { "Remove @package tags" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match @package and @subpackage lines
        let re = Regex::new(r"(?m)^\s*\*\s*@(?:package|subpackage)[^\n]*\n").unwrap();

        for m in re.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(),
                String::new(),
                "Remove @package tag".to_string(),
                "phpdoc_no_package",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_no_package() {
        let code = "<?php
/**
 * @package Foo
 */";
        let edits = PhpdocNoPackageFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
