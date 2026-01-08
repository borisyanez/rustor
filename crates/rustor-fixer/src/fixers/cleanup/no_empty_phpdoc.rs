//! Remove empty PHPDoc blocks

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoEmptyPhpdocFixer;

impl Fixer for NoEmptyPhpdocFixer {
    fn name(&self) -> &'static str { "no_empty_phpdoc" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_empty_phpdoc" }
    fn description(&self) -> &'static str { "Remove empty PHPDoc blocks" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match empty /** */ or /** \n * \n */ blocks
        let re = Regex::new(r"(?ms)/\*\*\s*(\*\s*)*\*/\s*\n?").unwrap();

        for m in re.find_iter(source) {
            let content = m.as_str();
            // Check if there's any actual content besides * and whitespace
            let stripped: String = content.chars()
                .filter(|c| !c.is_whitespace() && *c != '*' && *c != '/')
                .collect();

            if stripped.is_empty() {
                edits.push(edit_with_rule(
                    m.start(), m.end(), String::new(),
                    "Remove empty PHPDoc".to_string(),
                    "no_empty_phpdoc",
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
    fn test_empty_phpdoc() {
        let edits = NoEmptyPhpdocFixer.check("<?php\n/**\n */\nfunction f() {}", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_non_empty_phpdoc() {
        let edits = NoEmptyPhpdocFixer.check("<?php\n/**\n * @return void\n */\nfunction f() {}", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
