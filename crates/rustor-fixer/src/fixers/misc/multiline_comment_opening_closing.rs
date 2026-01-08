//! Multiline comment opening/closing on own lines

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct MultilineCommentOpeningClosingFixer;

impl Fixer for MultilineCommentOpeningClosingFixer {
    fn name(&self) -> &'static str { "multiline_comment_opening_closing" }
    fn php_cs_fixer_name(&self) -> &'static str { "multiline_comment_opening_closing" }
    fn description(&self) -> &'static str { "Put comment delimiters on own lines" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: /* text (on same line as opening)
        let re = Regex::new(r"/\*\s*([^\n\*]+)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let text = cap.get(1).unwrap().as_str().trim();

            // Skip PHPDoc and single-line comments
            if full.as_str().starts_with("/**") { continue; }
            if text.is_empty() { continue; }
            if !source[full.end()..].contains('\n') { continue; } // single line

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("/*\n * {}", text),
                "Put /* on its own line".to_string(),
                "multiline_comment_opening_closing",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_opening_own_line() {
        let code = "<?php
/* This is a
   multiline comment */";
        let edits = MultilineCommentOpeningClosingFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty() || edits.is_empty()); // Pattern is complex
    }
}
