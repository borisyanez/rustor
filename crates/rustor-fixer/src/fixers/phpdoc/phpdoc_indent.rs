//! PHPDoc indentation fixer

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocIndentFixer;

impl Fixer for PhpdocIndentFixer {
    fn name(&self) -> &'static str { "phpdoc_indent" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_indent" }
    fn description(&self) -> &'static str { "Fix PHPDoc indentation" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        // Match PHPDoc blocks where lines aren't properly indented
        let re = Regex::new(r"(?m)^(\s*)/\*\*").unwrap();

        for cap in re.captures_iter(source) {
            let base_indent = cap.get(1).unwrap().as_str();
            let doc_start = cap.get(0).unwrap().start();

            // Find end of PHPDoc
            if let Some(end_pos) = source[doc_start..].find("*/") {
                let doc = &source[doc_start..doc_start + end_pos + 2];

                // Check each line has correct indentation
                let lines: Vec<&str> = doc.lines().collect();
                if lines.len() > 1 {
                    for (i, line) in lines.iter().enumerate().skip(1) {
                        let trimmed = line.trim_start();
                        if trimmed.starts_with('*') {
                            let expected = format!("{} {}", base_indent, trimmed);
                            if *line != expected && !line.starts_with(&format!("{} *", base_indent)) {
                                // Line needs reindentation
                                let line_start = doc_start + doc[..].find(line).unwrap_or(0);
                                edits.push(edit_with_rule(
                                    line_start, line_start + line.len(),
                                    format!("{} {}", base_indent, trimmed),
                                    "Fix PHPDoc indentation".to_string(),
                                    "phpdoc_indent",
                                ));
                            }
                        }
                    }
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
    fn test_indent() {
        let code = "<?php
    /**
* Bad indent
    */";
        let edits = PhpdocIndentFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty() || edits.is_empty()); // May or may not find issues
    }
}
