//! Remove trailing whitespace from comments

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes trailing whitespace from inline comments
pub struct NoTrailingWhitespaceInCommentFixer;

impl Fixer for NoTrailingWhitespaceInCommentFixer {
    fn name(&self) -> &'static str {
        "no_trailing_whitespace_in_comment"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_trailing_whitespace_in_comment"
    }

    fn description(&self) -> &'static str {
        "Remove trailing whitespace from comments"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let mut in_block_comment = false;
        let mut offset = 0;

        for line in source.lines() {
            let line_len = line.len();

            // Check if we're starting or ending a block comment
            if line.contains("/*") {
                in_block_comment = true;
            }
            if line.contains("*/") {
                in_block_comment = false;
            }

            // Check for trailing whitespace in comments
            if in_block_comment || line.trim_start().starts_with("//") || line.trim_start().starts_with("#") {
                let trimmed_len = line.trim_end().len();
                if trimmed_len < line_len {
                    edits.push(edit_with_rule(
                        offset + trimmed_len,
                        offset + line_len,
                        String::new(),
                        "Remove trailing whitespace in comment".to_string(),
                        "no_trailing_whitespace_in_comment",
                    ));
                }
            }

            // Also check doc comment lines (starting with *)
            let trimmed_start = line.trim_start();
            if trimmed_start.starts_with("*") && !trimmed_start.starts_with("*/") {
                let trimmed_len = line.trim_end().len();
                if trimmed_len < line_len {
                    // Only add if not already added
                    let already_added = edits.iter().any(|e| e.start_offset() == offset + trimmed_len);
                    if !already_added {
                        edits.push(edit_with_rule(
                            offset + trimmed_len,
                            offset + line_len,
                            String::new(),
                            "Remove trailing whitespace in comment".to_string(),
                            "no_trailing_whitespace_in_comment",
                        ));
                    }
                }
            }

            offset += line_len + 1; // +1 for newline
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        NoTrailingWhitespaceInCommentFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_no_trailing_whitespace() {
        let source = "<?php\n// comment\n$a = 1;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_trailing_whitespace_in_single_line_comment() {
        let source = "<?php\n// comment   \n$a = 1;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_trailing_whitespace_in_hash_comment() {
        let source = "<?php\n# comment   \n$a = 1;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_trailing_whitespace_in_block_comment() {
        let source = "<?php\n/* comment   \n * more   \n */\n";
        let edits = check(source);

        assert!(edits.len() >= 1);
    }

    #[test]
    fn test_trailing_whitespace_in_doc_comment() {
        let source = "<?php\n/**\n * Doc comment   \n */\n";
        let edits = check(source);

        assert!(edits.len() >= 1);
    }

    #[test]
    fn test_skip_non_comment_lines() {
        let source = "<?php\n$a = 1;   \n// comment\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
