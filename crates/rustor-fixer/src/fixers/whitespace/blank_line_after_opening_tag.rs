//! Ensure blank line after opening PHP tag

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures a blank line after the opening PHP tag when followed by namespace/use/class
pub struct BlankLineAfterOpeningTagFixer;

impl Fixer for BlankLineAfterOpeningTagFixer {
    fn name(&self) -> &'static str {
        "blank_line_after_opening_tag"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "blank_line_after_opening_tag"
    }

    fn description(&self) -> &'static str {
        "Ensure blank line after opening PHP tag"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        // Find <?php tag
        let php_tag = if source.starts_with("<?php") {
            Some(0)
        } else {
            source.find("<?php")
        };

        let Some(tag_start) = php_tag else {
            return vec![];
        };

        let tag_end = tag_start + 5; // "<?php" length

        // Find what comes after the tag
        let after_tag = &source[tag_end..];

        // Skip whitespace to find first non-whitespace
        let first_content_offset = after_tag
            .char_indices()
            .find(|(_, c)| !c.is_whitespace())
            .map(|(i, _)| i);

        let Some(content_offset) = first_content_offset else {
            // File has no content after <?php
            return vec![];
        };

        // Count newlines in whitespace between tag and content
        let whitespace_between = &after_tag[..content_offset];
        let newline_count = whitespace_between.matches('\n').count();

        // PSR-12 requires blank line after opening tag if followed by namespace/use/declare
        let content_after = after_tag[content_offset..].trim_start();
        let needs_blank_line = content_after.starts_with("namespace ")
            || content_after.starts_with("use ")
            || content_after.starts_with("declare(")
            || content_after.starts_with("declare ");

        if needs_blank_line && newline_count < 2 {
            // Need to insert blank line
            let line_ending = config.line_ending.as_str();
            let insert_pos = tag_end;

            // Replace all whitespace after tag with correct spacing
            let replacement = format!("{}{}", line_ending, line_ending);

            vec![edit_with_rule(
                insert_pos,
                tag_end + content_offset,
                replacement,
                "Add blank line after opening PHP tag".to_string(),
                "blank_line_after_opening_tag",
            )]
        } else if !needs_blank_line && newline_count > 1 {
            // May want to remove extra blank lines for simple scripts
            // But per PSR-12, we only enforce adding, not removing
            vec![]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IndentStyle, LineEnding};

    fn check(source: &str) -> Vec<Edit> {
        BlankLineAfterOpeningTagFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            indent: IndentStyle::default(),
            options: Default::default(),
        })
    }

    #[test]
    fn test_correct_blank_line_after_namespace() {
        let source = "<?php\n\nnamespace App;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_missing_blank_line_before_namespace() {
        let source = "<?php\nnamespace App;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("\n\n"));
    }

    #[test]
    fn test_no_blank_line_needed_for_simple_script() {
        let source = "<?php\n$a = 1;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_blank_line_before_use() {
        let source = "<?php\nuse App\\Service;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_blank_line_before_declare() {
        let source = "<?php\ndeclare(strict_types=1);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_correct_with_declare() {
        let source = "<?php\n\ndeclare(strict_types=1);\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_inline_content() {
        let source = "<?php echo 'hi';";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_no_php_tag() {
        let source = "<html><body>Test</body></html>";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
