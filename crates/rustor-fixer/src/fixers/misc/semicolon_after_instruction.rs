//! Add semicolon after standalone instructions

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct SemicolonAfterInstructionFixer;

impl Fixer for SemicolonAfterInstructionFixer {
    fn name(&self) -> &'static str { "semicolon_after_instruction" }
    fn php_cs_fixer_name(&self) -> &'static str { "semicolon_after_instruction" }
    fn description(&self) -> &'static str { "Add semicolon after instruction" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match PHP close tag without preceding semicolon
        // Pattern: statement without ; before ?>
        let re = Regex::new(r"([^;\s{}\n])\s*\?>").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let last_char = cap.get(1).unwrap().as_str();

            // Don't add if it's already a special case
            if last_char == ":" || last_char == "/" { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}; ?>", last_char),
                "Add semicolon before ?>".to_string(),
                "semicolon_after_instruction",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_semicolon() {
        let edits = SemicolonAfterInstructionFixer.check("<?php echo 'hi' ?>", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_semicolon_present() {
        let edits = SemicolonAfterInstructionFixer.check("<?php echo 'hi'; ?>", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
