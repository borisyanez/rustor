//! Remove whitespace from blank lines

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes whitespace from blank lines
pub struct NoWhitespaceInBlankLineFixer;

impl Fixer for NoWhitespaceInBlankLineFixer {
    fn name(&self) -> &'static str {
        "no_whitespace_in_blank_line"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_whitespace_in_blank_line"
    }

    fn description(&self) -> &'static str {
        "Remove whitespace from blank lines"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let mut offset = 0;

        for (line_num, line) in source.lines().enumerate() {
            // A blank line is one that contains only whitespace
            if !line.is_empty() && line.trim().is_empty() {
                edits.push(edit_with_rule(
                    offset,
                    offset + line.len(),
                    String::new(),
                    format!("Remove whitespace from blank line {}", line_num + 1),
                    "no_whitespace_in_blank_line",
                ));
            }

            // Move to next line
            offset += line.len();
            if offset < source.len() {
                if source[offset..].starts_with("\r\n") {
                    offset += 2;
                } else if source[offset..].starts_with('\n') || source[offset..].starts_with('\r') {
                    offset += 1;
                }
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        NoWhitespaceInBlankLineFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_no_blank_lines() {
        let edits = check("<?php\n$a = 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_truly_blank_line() {
        let source = "<?php\n\n$a = 1;\n";
        let edits = check(source);
        // Empty line is fine, nothing to remove
        assert!(edits.is_empty());
    }

    #[test]
    fn test_blank_line_with_spaces() {
        let source = "<?php\n   \n$a = 1;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "");
    }

    #[test]
    fn test_blank_line_with_tabs() {
        let source = "<?php\n\t\t\n$a = 1;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_blank_lines_with_whitespace() {
        let source = "<?php\n   \n\t\n$a = 1;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_line_with_content_not_affected() {
        let source = "<?php\n   $a = 1;   \n";
        let edits = check(source);

        // This is not a blank line (has content)
        assert!(edits.is_empty());
    }
}
