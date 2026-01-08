//! Rename PHPDoc tags

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct GeneralPhpdocTagRenameFixer;

impl Fixer for GeneralPhpdocTagRenameFixer {
    fn name(&self) -> &'static str { "general_phpdoc_tag_rename" }
    fn php_cs_fixer_name(&self) -> &'static str { "general_phpdoc_tag_rename" }
    fn description(&self) -> &'static str { "Rename PHPDoc tags" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Common tag renames
        let renames = [
            ("@inheritdoc", "@inheritDoc"),
            ("@inheritDocs", "@inheritDoc"),
            ("@link", "@see"),
        ];

        for (from, to) in &renames {
            let re = Regex::new(&format!(r"(?m)^\s*\*\s*{}\b", regex::escape(from))).unwrap();

            for m in re.find_iter(source) {
                let matched = m.as_str();
                let new_text = matched.replace(from, to);

                if matched != new_text {
                    edits.push(edit_with_rule(
                        m.start(), m.end(),
                        new_text,
                        format!("Rename {} to {}", from, to),
                        "general_phpdoc_tag_rename",
                    ));
                }
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_inheritdoc_rename() {
        let code = "<?php\n/**\n * @inheritdoc\n */";
        let edits = GeneralPhpdocTagRenameFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
