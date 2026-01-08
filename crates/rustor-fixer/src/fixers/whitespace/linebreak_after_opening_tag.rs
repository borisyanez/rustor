//! Linebreak after opening tag fixer
//!
//! Ensures there is a linebreak after the PHP opening tag.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures linebreak after opening PHP tag
pub struct LinebreakAfterOpeningTagFixer;

impl Fixer for LinebreakAfterOpeningTagFixer {
    fn name(&self) -> &'static str {
        "linebreak_after_opening_tag"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "linebreak_after_opening_tag"
    }

    fn description(&self) -> &'static str {
        "Ensure linebreak after opening PHP tag"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Match <?php followed by non-newline content on same line
        // But not <?php followed by newline (already correct)
        let re = Regex::new(r"<\?php[ \t]+([^\n\r])").unwrap();

        if let Some(cap) = re.captures(source) {
            let full_match = cap.get(0).unwrap();

            // Get position after <?php and any space
            let php_tag_end = full_match.start() + 5; // "<?php" is 5 chars

            // Find where the actual code starts (after whitespace)
            let after_tag = &source[php_tag_end..full_match.end()];
            let spaces = after_tag.len() - 1; // -1 for the captured char

            // Replace space with newline
            edits.push(edit_with_rule(
                php_tag_end,
                php_tag_end + spaces,
                line_ending.to_string(),
                "Add linebreak after opening PHP tag".to_string(),
                "linebreak_after_opening_tag",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LineEnding;

    fn check(source: &str) -> Vec<Edit> {
        LinebreakAfterOpeningTagFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        })
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\necho 'hello';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_same_line_code() {
        let source = "<?php echo 'hello';\n";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "\n");
    }

    #[test]
    fn test_multiple_spaces() {
        let source = "<?php    echo 'hello';\n";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_short_tag_unchanged() {
        // Short tags are different
        let source = "<?= $var ?>\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
