//! Convert useless PHPDoc to regular comment

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocToCommentFixer;

impl Fixer for PhpdocToCommentFixer {
    fn name(&self) -> &'static str { "phpdoc_to_comment" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_to_comment" }
    fn description(&self) -> &'static str { "Convert useless PHPDoc to comment" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match PHPDoc without any @ tags (just text)
        let re = Regex::new(r"(?ms)/\*\*\s*\n(\s*\*\s*[^@\n][^\n]*\n)*\s*\*/").unwrap();

        for m in re.find_iter(source) {
            let doc = m.as_str();

            // Check if there are any @ tags
            if !doc.contains('@') {
                // Check if followed by something that needs PHPDoc
                let after = &source[m.end()..];
                let trimmed = after.trim_start();

                // If not followed by class/function/property, convert to regular comment
                if !trimmed.starts_with("class ")
                    && !trimmed.starts_with("function ")
                    && !trimmed.starts_with("public ")
                    && !trimmed.starts_with("protected ")
                    && !trimmed.starts_with("private ")
                    && !trimmed.starts_with("abstract ")
                    && !trimmed.starts_with("final ")
                {
                    let converted = doc.replace("/**", "/*");
                    edits.push(edit_with_rule(
                        m.start(), m.end(),
                        converted,
                        "Convert PHPDoc to regular comment".to_string(),
                        "phpdoc_to_comment",
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
    fn test_useless_phpdoc() {
        let code = "<?php\n/**\n * Just a comment\n */\n$a = 1;";
        let edits = PhpdocToCommentFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_useful_phpdoc() {
        let code = "<?php\n/**\n * @param int $x\n */\nfunction f($x) {}";
        let edits = PhpdocToCommentFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
