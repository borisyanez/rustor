//! Remove whitespace before namespace declaration

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoLeadingNamespaceWhitespaceFixer;

impl Fixer for NoLeadingNamespaceWhitespaceFixer {
    fn name(&self) -> &'static str { "no_leading_namespace_whitespace" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_leading_namespace_whitespace" }
    fn description(&self) -> &'static str { "No whitespace before namespace" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match namespace with leading whitespace on the line
        let re = Regex::new(r"(?m)^([ \t]+)(namespace\s)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let ns = cap.get(2).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                ns.to_string(),
                "Remove leading whitespace before namespace".to_string(),
                "no_leading_namespace_whitespace",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_leading_whitespace() {
        let code = "<?php\n    namespace App;";
        let edits = NoLeadingNamespaceWhitespaceFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_no_leading_whitespace() {
        let code = "<?php\nnamespace App;";
        let edits = NoLeadingNamespaceWhitespaceFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
