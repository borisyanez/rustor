//! Remove closing PHP tag from pure PHP files

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes the closing `?>` tag from files containing only PHP
pub struct NoClosingTagFixer;

impl Fixer for NoClosingTagFixer {
    fn name(&self) -> &'static str {
        "no_closing_tag"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_closing_tag"
    }

    fn description(&self) -> &'static str {
        "Remove closing ?> tag from pure PHP files"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        // Only process if file starts with <?php
        if !source.trim_start().starts_with("<?php") {
            return vec![];
        }

        // Find last ?> in file
        let trimmed = source.trim_end();
        if !trimmed.ends_with("?>") {
            return vec![];
        }

        // Find the position of the last ?>
        let close_tag_start = trimmed.rfind("?>").unwrap();

        // Check if there's any HTML after the opening PHP tag
        // If there is, we shouldn't remove the closing tag
        let after_php_open = source.find("<?php")
            .map(|i| i + 5)
            .unwrap_or(0);

        // Check for any ?> followed by content that isn't just the final ?>
        let has_mixed_content = source[after_php_open..close_tag_start]
            .contains("?>");

        if has_mixed_content {
            return vec![];
        }

        // Remove the closing tag and any trailing whitespace before it
        let mut remove_start = close_tag_start;

        // Also remove whitespace/newlines before ?>
        while remove_start > 0 {
            let prev = source[..remove_start].chars().last();
            match prev {
                Some(' ') | Some('\t') => remove_start -= 1,
                Some('\n') => {
                    remove_start -= 1;
                    // Also check for \r before \n
                    if remove_start > 0 && source[..remove_start].ends_with('\r') {
                        remove_start -= 1;
                    }
                    break; // Keep one newline before
                }
                Some('\r') => {
                    remove_start -= 1;
                    break;
                }
                _ => break,
            }
        }

        // Ensure file ends with newline
        let line_ending = config.line_ending.as_str();
        let replacement = line_ending.to_string();

        vec![edit_with_rule(
            remove_start,
            source.len(),
            replacement,
            "Remove closing PHP tag".to_string(),
            "no_closing_tag",
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IndentStyle, LineEnding};

    fn check(source: &str) -> Vec<Edit> {
        NoClosingTagFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            indent: IndentStyle::default(),
            options: Default::default(),
        })
    }

    #[test]
    fn test_no_closing_tag_unchanged() {
        let edits = check("<?php\n$a = 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_remove_closing_tag() {
        let source = "<?php\n$a = 1;\n?>";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "\n");
    }

    #[test]
    fn test_remove_closing_tag_with_whitespace() {
        let source = "<?php\n$a = 1;\n   ?>";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_html_unchanged() {
        // Don't remove ?> if there's HTML content
        let source = "<?php $a = 1; ?>\n<html></html>\n<?php $b = 2; ?>";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_html_file_unchanged() {
        let source = "<html>\n<?php echo 'hi'; ?>\n</html>";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_closing_tag_with_newline() {
        let source = "<?php\n$a = 1;\n?>\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }
}
