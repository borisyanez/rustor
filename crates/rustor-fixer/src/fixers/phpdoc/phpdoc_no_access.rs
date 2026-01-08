//! Remove @access tags from PHPDoc

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocNoAccessFixer;

impl Fixer for PhpdocNoAccessFixer {
    fn name(&self) -> &'static str { "phpdoc_no_access" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_no_access" }
    fn description(&self) -> &'static str { "Remove @access tags" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match @access public/protected/private
        let re = Regex::new(r"(?m)^\s*\*\s*@access\s+(public|protected|private)\s*\n").unwrap();

        for m in re.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(),
                String::new(),
                "Remove @access tag".to_string(),
                "phpdoc_no_access",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_access_tag() {
        let code = "<?php\n/**\n * @access public\n */";
        let edits = PhpdocNoAccessFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_no_access_tag() {
        let code = "<?php\n/**\n * @param int $x\n */";
        let edits = PhpdocNoAccessFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
