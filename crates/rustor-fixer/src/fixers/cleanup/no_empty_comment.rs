//! Remove empty comments

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoEmptyCommentFixer;

impl Fixer for NoEmptyCommentFixer {
    fn name(&self) -> &'static str { "no_empty_comment" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_empty_comment" }
    fn description(&self) -> &'static str { "Remove empty comments" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match empty // comments
        let re1 = Regex::new(r"//\s*\n").unwrap();
        for m in re1.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(), String::new(),
                "Remove empty comment".to_string(),
                "no_empty_comment",
            ));
        }

        // Match empty # comments
        let re2 = Regex::new(r"#\s*\n").unwrap();
        for m in re2.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(), String::new(),
                "Remove empty comment".to_string(),
                "no_empty_comment",
            ));
        }

        // Match empty /* */ comments
        let re3 = Regex::new(r"/\*\s*\*/").unwrap();
        for m in re3.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(), String::new(),
                "Remove empty comment".to_string(),
                "no_empty_comment",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_empty_single_line_comment() {
        let edits = NoEmptyCommentFixer.check("<?php\n//\n$a = 1;", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_empty_block_comment() {
        let edits = NoEmptyCommentFixer.check("<?php\n/**/\n$a = 1;", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_non_empty_comment_unchanged() {
        let edits = NoEmptyCommentFixer.check("<?php\n// hello\n$a = 1;", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
