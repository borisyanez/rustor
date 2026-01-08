//! Convert isset ternary to null coalescing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct TernaryToNullCoalescingFixer;

impl Fixer for TernaryToNullCoalescingFixer {
    fn name(&self) -> &'static str { "ternary_to_null_coalescing" }
    fn php_cs_fixer_name(&self) -> &'static str { "ternary_to_null_coalescing" }
    fn description(&self) -> &'static str { "Convert isset ternary to ??" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: isset($x) ? $x : $default
        let re = Regex::new(r"isset\s*\((\$\w+)\)\s*\?\s*(\$\w+)\s*:\s*([^;]+)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let isset_var = cap.get(1).unwrap().as_str();
            let then_var = cap.get(2).unwrap().as_str();
            let else_val = cap.get(3).unwrap().as_str().trim();

            // Only convert if isset var matches then var
            if isset_var == then_var {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{} ?? {}", isset_var, else_val),
                    "Use null coalescing operator".to_string(),
                    "ternary_to_null_coalescing",
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
    fn test_ternary_to_coalesce() {
        let edits = TernaryToNullCoalescingFixer.check("<?php\n$a = isset($x) ? $x : 'default';", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("$x ?? 'default'"));
    }

    #[test]
    fn test_different_vars_unchanged() {
        let edits = TernaryToNullCoalescingFixer.check("<?php\n$a = isset($x) ? $y : 'default';", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
