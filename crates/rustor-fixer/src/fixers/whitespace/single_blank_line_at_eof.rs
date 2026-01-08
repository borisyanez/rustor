//! Ensure single blank line at end of file

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures file ends with exactly one blank line
pub struct SingleBlankLineAtEofFixer;

impl Fixer for SingleBlankLineAtEofFixer {
    fn name(&self) -> &'static str {
        "single_blank_line_at_eof"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "single_blank_line_at_end_of_file"
    }

    fn description(&self) -> &'static str {
        "Ensure file ends with exactly one newline"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        if source.is_empty() {
            return vec![];
        }

        let line_ending = config.line_ending.as_str();

        // Count trailing newlines (for reference)
        let _trailing_newlines = source
            .chars()
            .rev()
            .take_while(|c| *c == '\n' || *c == '\r')
            .count();

        // Find where content ends (excluding trailing whitespace/newlines)
        let content_end = source.trim_end().len();

        if content_end == 0 {
            // File is all whitespace
            return vec![];
        }

        let current_trailing = &source[content_end..];

        // Check if we need to make changes
        let needs_fix = if line_ending == "\n" {
            // For LF: should end with exactly "\n"
            current_trailing != "\n"
        } else {
            // For CRLF: should end with exactly "\r\n"
            current_trailing != "\r\n"
        };

        if needs_fix {
            vec![edit_with_rule(
                content_end,
                source.len(),
                line_ending.to_string(),
                "Ensure single newline at end of file".to_string(),
                "single_blank_line_at_end_of_file",
            )]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IndentStyle, LineEnding};

    fn check_lf(source: &str) -> Vec<Edit> {
        SingleBlankLineAtEofFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            indent: IndentStyle::default(),
            options: Default::default(),
        })
    }

    fn check_crlf(source: &str) -> Vec<Edit> {
        SingleBlankLineAtEofFixer.check(source, &FixerConfig {
            line_ending: LineEnding::CrLf,
            indent: IndentStyle::default(),
            options: Default::default(),
        })
    }

    #[test]
    fn test_correct_ending_lf() {
        let edits = check_lf("<?php\n$a = 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_correct_ending_crlf() {
        let edits = check_crlf("<?php\r\n$a = 1;\r\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_no_trailing_newline() {
        let source = "<?php\n$a = 1;";
        let edits = check_lf(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "\n");
    }

    #[test]
    fn test_multiple_trailing_newlines() {
        let source = "<?php\n$a = 1;\n\n\n";
        let edits = check_lf(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "\n");
    }

    #[test]
    fn test_trailing_whitespace_and_newlines() {
        let source = "<?php\n$a = 1;   \n\n";
        let edits = check_lf(source);

        // Should normalize to single newline after content
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_crlf_to_single() {
        let source = "<?php\r\n$a = 1;\r\n\r\n";
        let edits = check_crlf(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "\r\n");
    }

    #[test]
    fn test_empty_file() {
        let edits = check_lf("");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_whitespace_only_file() {
        let edits = check_lf("   \n\n");
        assert!(edits.is_empty());
    }
}
