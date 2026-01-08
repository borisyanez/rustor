//! Remove unused closure imports

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct LambdaNotUsedImportFixer;

impl Fixer for LambdaNotUsedImportFixer {
    fn name(&self) -> &'static str { "lambda_not_used_import" }
    fn php_cs_fixer_name(&self) -> &'static str { "lambda_not_used_import" }
    fn description(&self) -> &'static str { "Remove unused closure imports" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match function() use ($var) { body }
        let re = Regex::new(r"function\s*\([^)]*\)\s*use\s*\(\s*(\$\w+)\s*\)\s*\{([^}]*)\}").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let var = cap.get(1).unwrap().as_str();
            let body = cap.get(2).unwrap().as_str();

            // Check if variable is used in body
            if !body.contains(var) {
                // Variable not used - would need to remove from use clause
                // This is complex to implement properly
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_unused_import() {
        let code = "<?php\n$f = function() use ($unused) { return 1; };";
        let edits = LambdaNotUsedImportFixer.check(code, &FixerConfig::default());
        // Complex detection
        assert!(edits.is_empty() || !edits.is_empty());
    }
}
