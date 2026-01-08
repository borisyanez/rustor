//! Simplify null return in void functions

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct SimplifiedNullReturnFixer;

impl Fixer for SimplifiedNullReturnFixer {
    fn name(&self) -> &'static str { "simplified_null_return" }
    fn php_cs_fixer_name(&self) -> &'static str { "simplified_null_return" }
    fn description(&self) -> &'static str { "Simplify return null in void functions" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: return null; in function with : void return type
        // Look for void return type, then find return null; statements
        let func_re = Regex::new(r"(?ms)function\s+\w+\s*\([^)]*\)\s*:\s*void\s*\{([\s\S]*?)\}").unwrap();

        for cap in func_re.captures_iter(source) {
            let body_match = cap.get(1).unwrap();
            let body = body_match.as_str();
            let body_start = body_match.start();

            // Find return null; in the body
            let return_re = Regex::new(r"return\s+null\s*;").unwrap();
            for m in return_re.find_iter(body) {
                edits.push(edit_with_rule(
                    body_start + m.start(), body_start + m.end(),
                    "return;".to_string(),
                    "Simplify return null to return in void function".to_string(),
                    "simplified_null_return",
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
    fn test_return_null_in_void() {
        let code = "<?php
function foo(): void {
    return null;
}";
        let edits = SimplifiedNullReturnFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "return;");
    }

    #[test]
    fn test_return_null_non_void_unchanged() {
        let code = "<?php
function foo(): ?string {
    return null;
}";
        let edits = SimplifiedNullReturnFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
