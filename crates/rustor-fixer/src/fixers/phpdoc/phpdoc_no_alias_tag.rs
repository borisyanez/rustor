//! Replace PHPDoc alias tags with canonical versions

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocNoAliasTagFixer;

impl Fixer for PhpdocNoAliasTagFixer {
    fn name(&self) -> &'static str { "phpdoc_no_alias_tag" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_no_alias_tag" }
    fn description(&self) -> &'static str { "Use canonical PHPDoc tags" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Alias -> Canonical mappings
        let aliases = [
            ("@type", "@var"),
            ("@link", "@see"),
            ("@property-read", "@property"),
            ("@property-write", "@property"),
        ];

        for (alias, canonical) in &aliases {
            let re = Regex::new(&format!(r"(?m)^(\s*\*\s*){}", regex::escape(alias))).unwrap();

            for cap in re.captures_iter(source) {
                let full = cap.get(0).unwrap();
                let prefix = cap.get(1).unwrap().as_str();

                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{}{}", prefix, canonical),
                    format!("Use {} instead of {}", canonical, alias),
                    "phpdoc_no_alias_tag",
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
    fn test_type_to_var() {
        let code = "<?php
/**
 * @type int
 */";
        let edits = PhpdocNoAliasTagFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("@var"));
    }
}
