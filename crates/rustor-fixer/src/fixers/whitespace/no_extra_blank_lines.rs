//! Remove extra blank lines

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoExtraBlankLinesFixer;

impl Fixer for NoExtraBlankLinesFixer {
    fn name(&self) -> &'static str { "no_extra_blank_lines" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_extra_blank_lines" }
    fn description(&self) -> &'static str { "Remove extra blank lines" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match 3+ consecutive newlines (2+ blank lines)
        let re = Regex::new(r"\n\n\n+").unwrap();

        for m in re.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(),
                "\n\n".to_string(),
                "Remove extra blank lines".to_string(),
                "no_extra_blank_lines",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_extra_blank_lines() {
        let code = "<?php\n\n\n\n$a = 1;";
        let edits = NoExtraBlankLinesFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "\n\n");
    }

    #[test]
    fn test_single_blank_unchanged() {
        let code = "<?php\n\n$a = 1;";
        let edits = NoExtraBlankLinesFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
