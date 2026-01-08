//! Remove unneeded import aliases

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoUnneededImportAliasFixer;

impl Fixer for NoUnneededImportAliasFixer {
    fn name(&self) -> &'static str { "no_unneeded_import_alias" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_unneeded_import_alias" }
    fn description(&self) -> &'static str { "Remove redundant use aliases" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: use Foo\Bar as Bar;  (alias same as class name)
        let re = Regex::new(r"(?m)^(\s*use\s+[A-Za-z0-9_\\]+\\)([A-Za-z0-9_]+)\s+as\s+([A-Za-z0-9_]+)\s*;").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let prefix = cap.get(1).unwrap().as_str();
            let class_name = cap.get(2).unwrap().as_str();
            let alias = cap.get(3).unwrap().as_str();

            if class_name == alias {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{}{};", prefix, class_name),
                    "Remove redundant alias".to_string(),
                    "no_unneeded_import_alias",
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
    fn test_redundant_alias() {
        let edits = NoUnneededImportAliasFixer.check("<?php\nuse Foo\\Bar as Bar;", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("use Foo\\Bar;"));
    }

    #[test]
    fn test_different_alias_unchanged() {
        let edits = NoUnneededImportAliasFixer.check("<?php\nuse Foo\\Bar as Baz;", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
