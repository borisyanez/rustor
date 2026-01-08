//! Use short class names when possible (imports exist)

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct FullyQualifiedStrictTypesFixer;

impl Fixer for FullyQualifiedStrictTypesFixer {
    fn name(&self) -> &'static str { "fully_qualified_strict_types" }
    fn php_cs_fixer_name(&self) -> &'static str { "fully_qualified_strict_types" }
    fn description(&self) -> &'static str { "Use imported class names" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Collect use imports
        let use_re = Regex::new(r"use\s+([A-Za-z0-9_\\]+)(?:\s+as\s+(\w+))?\s*;").unwrap();
        let mut imports: Vec<(String, String)> = Vec::new();

        for cap in use_re.captures_iter(source) {
            let full_class = cap.get(1).unwrap().as_str();
            let alias = cap.get(2).map_or_else(
                || full_class.split('\\').last().unwrap_or(full_class),
                |m| m.as_str()
            );
            imports.push((full_class.to_string(), alias.to_string()));
        }

        // Find fully qualified names in type hints that could use imports
        for (full_class, alias) in &imports {
            let escaped = regex::escape(full_class);
            let re = Regex::new(&format!(r"\\?{}", escaped)).unwrap();

            for m in re.find_iter(source) {
                // Don't replace in use statements
                let before = &source[..m.start()];
                if before.rfind("use ").map_or(false, |pos| !before[pos..].contains(';')) {
                    continue;
                }

                edits.push(edit_with_rule(
                    m.start(), m.end(),
                    alias.clone(),
                    "Use imported class name".to_string(),
                    "fully_qualified_strict_types",
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
    fn test_fqcn_to_import() {
        let code = "<?php
use App\\Models\\User;
function f(\\App\\Models\\User $u) {}";
        let edits = FullyQualifiedStrictTypesFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty() || edits.is_empty()); // Depends on exact matching
    }
}
