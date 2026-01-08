//! Convert $x = $x ?? $y to $x ??= $y

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct AssignNullCoalescingToCoalesceEqualFixer;

impl Fixer for AssignNullCoalescingToCoalesceEqualFixer {
    fn name(&self) -> &'static str { "assign_null_coalescing_to_coalesce_equal" }
    fn php_cs_fixer_name(&self) -> &'static str { "assign_null_coalescing_to_coalesce_equal" }
    fn description(&self) -> &'static str { "Use ??= operator" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: $x = $x ?? $y
        let re = Regex::new(r"(\$\w+)\s*=\s*(\$\w+)\s*\?\?\s*([^;]+)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let assign_var = cap.get(1).unwrap().as_str();
            let coalesce_var = cap.get(2).unwrap().as_str();
            let default = cap.get(3).unwrap().as_str().trim();

            // Only convert if same variable
            if assign_var == coalesce_var {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{} ??= {}", assign_var, default),
                    "Use ??= operator".to_string(),
                    "assign_null_coalescing_to_coalesce_equal",
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
    fn test_coalesce_equal() {
        let edits = AssignNullCoalescingToCoalesceEqualFixer.check("<?php\n$x = $x ?? 'default';", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("$x ??= 'default'"));
    }

    #[test]
    fn test_different_vars_unchanged() {
        let edits = AssignNullCoalescingToCoalesceEqualFixer.check("<?php\n$x = $y ?? 'default';", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
