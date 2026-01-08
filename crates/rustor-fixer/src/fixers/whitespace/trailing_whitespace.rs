//! Remove trailing whitespace from lines

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes trailing whitespace at the end of lines
pub struct TrailingWhitespaceFixer;

impl Fixer for TrailingWhitespaceFixer {
    fn name(&self) -> &'static str {
        "trailing_whitespace"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_trailing_whitespace"
    }

    fn description(&self) -> &'static str {
        "Remove trailing whitespace at the end of lines"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let mut offset = 0;

        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim_end();
            if trimmed.len() < line.len() {
                let trailing_start = offset + trimmed.len();
                let trailing_end = offset + line.len();

                edits.push(edit_with_rule(
                    trailing_start,
                    trailing_end,
                    String::new(),
                    format!("Remove trailing whitespace on line {}", line_num + 1),
                    "no_trailing_whitespace",
                ));
            }

            // Move offset to next line (account for \n or \r\n)
            offset += line.len();
            if offset < source.len() {
                // Check for line ending
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
        TrailingWhitespaceFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_no_trailing_whitespace() {
        let edits = check("<?php\n$a = 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_trailing_spaces() {
        let source = "<?php   \n$a = 1;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start_offset(), 5); // After "<?php"
        assert_eq!(edits[0].end_offset(), 8);   // Before \n
        assert_eq!(edits[0].replacement, "");
    }

    #[test]
    fn test_trailing_tabs() {
        let source = "<?php\t\t\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "");
    }

    #[test]
    fn test_multiple_lines() {
        let source = "<?php   \n$a = 1;  \n$b = 2;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_mixed_whitespace() {
        let source = "<?php \t \n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        // Should remove " \t "
        assert_eq!(edits[0].start_offset(), 5);
        assert_eq!(edits[0].end_offset(), 8);
    }

    #[test]
    fn test_preserves_content() {
        let source = "<?php\n$a = '   ';   \n";
        let edits = check(source);

        // Should only remove trailing, not content
        assert_eq!(edits.len(), 1);
    }
}
