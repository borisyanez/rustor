//! Simplify if/return to direct return

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct SimplifiedIfReturnFixer;

impl Fixer for SimplifiedIfReturnFixer {
    fn name(&self) -> &'static str { "simplified_if_return" }
    fn php_cs_fixer_name(&self) -> &'static str { "simplified_if_return" }
    fn description(&self) -> &'static str { "Simplify if/return to direct return" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: if ($cond) { return true; } return false;
        let re = Regex::new(r"(?ms)if\s*\(([^)]+)\)\s*\{\s*return\s+true\s*;\s*\}\s*return\s+false\s*;").unwrap();
        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let cond = cap.get(1).unwrap().as_str().trim();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("return {};", cond),
                "Simplify if/return to direct return".to_string(),
                "simplified_if_return",
            ));
        }

        // Match: if ($cond) { return false; } return true;
        let re2 = Regex::new(r"(?ms)if\s*\(([^)]+)\)\s*\{\s*return\s+false\s*;\s*\}\s*return\s+true\s*;").unwrap();
        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let cond = cap.get(1).unwrap().as_str().trim();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("return !({});", cond),
                "Simplify if/return to direct return".to_string(),
                "simplified_if_return",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_if_true_return_false() {
        let code = "<?php
if ($x > 0) {
    return true;
}
return false;";
        let edits = SimplifiedIfReturnFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("return $x > 0"));
    }

    #[test]
    fn test_if_false_return_true() {
        let code = "<?php
if ($x) {
    return false;
}
return true;";
        let edits = SimplifiedIfReturnFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("!"));
    }
}
