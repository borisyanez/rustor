//! No blank lines after PHPDoc

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoBlankLinesAfterPhpdocFixer;

impl Fixer for NoBlankLinesAfterPhpdocFixer {
    fn name(&self) -> &'static str { "no_blank_lines_after_phpdoc" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_blank_lines_after_phpdoc" }
    fn description(&self) -> &'static str { "No blank lines after PHPDoc" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match PHPDoc followed by blank lines (one or more extra newlines)
        let re = Regex::new(r"\*/\n(\n+)(\s*)(public|protected|private|function|class|interface|trait|abstract|final)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let _blanks = cap.get(1).unwrap();
            let indent = cap.get(2).unwrap().as_str();
            let keyword = cap.get(3).unwrap().as_str();

            // The regex already matches if there's at least one extra newline after */\n
            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("*/\n{}{}", indent, keyword),
                "Remove blank lines after PHPDoc".to_string(),
                "no_blank_lines_after_phpdoc",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_blank_after_phpdoc() {
        // Two blank lines after PHPDoc (which means 2+ newlines in the blanks group)
        let code = "<?php\n/**\n * Doc\n */\n\n\npublic function f() {}";
        let edits = NoBlankLinesAfterPhpdocFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_no_blank_after_phpdoc() {
        let code = "<?php\n/**\n * Doc\n */\npublic function f() {}";
        let edits = NoBlankLinesAfterPhpdocFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }

    #[test]
    fn test_single_blank_after_phpdoc() {
        // Single blank line should still trigger (the regex matches \n(\n+))
        let code = "<?php\n/**\n * Doc\n */\n\nfunction f() {}";
        let edits = NoBlankLinesAfterPhpdocFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
