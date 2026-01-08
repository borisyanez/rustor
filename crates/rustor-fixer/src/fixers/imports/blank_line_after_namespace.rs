//! Ensure blank line after namespace declaration

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures a blank line after namespace declaration
pub struct BlankLineAfterNamespaceFixer;

impl Fixer for BlankLineAfterNamespaceFixer {
    fn name(&self) -> &'static str {
        "blank_line_after_namespace"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "blank_line_after_namespace"
    }

    fn description(&self) -> &'static str {
        "Ensure blank line after namespace declaration"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Match namespace declaration followed by semicolon
        // Use [ \t]* instead of \s* to avoid consuming newlines
        let re = Regex::new(r"(?m)^namespace\s+[^;{]+;[ \t]*\n").unwrap();

        for mat in re.find_iter(source) {
            let end_pos = mat.end();

            // Check what follows the namespace line
            if end_pos >= source.len() {
                continue;
            }

            let after = &source[end_pos..];

            // Count leading newlines
            let leading_newlines = after.chars().take_while(|c| *c == '\n' || *c == '\r').count();

            // PSR-12 requires exactly one blank line (2 newlines total including the one after ;)
            // If we have the namespace line ending with \n and the next line starts immediately,
            // we need to add one blank line
            if leading_newlines == 0 {
                // Check if next line is not a closing brace or empty
                let next_line = after.lines().next().unwrap_or("");
                if !next_line.trim().is_empty() && !next_line.trim().starts_with('}') {
                    edits.push(edit_with_rule(
                        end_pos,
                        end_pos,
                        line_ending.to_string(),
                        "Add blank line after namespace".to_string(),
                        "blank_line_after_namespace",
                    ));
                }
            } else if leading_newlines > 1 {
                // Too many blank lines, reduce to one
                let mut newline_bytes = 0;
                for c in after.chars() {
                    if c == '\n' || c == '\r' {
                        newline_bytes += c.len_utf8();
                    } else {
                        break;
                    }
                }

                edits.push(edit_with_rule(
                    end_pos,
                    end_pos + newline_bytes,
                    line_ending.to_string(),
                    "Ensure single blank line after namespace".to_string(),
                    "blank_line_after_namespace",
                ));
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IndentStyle, LineEnding};

    fn check(source: &str) -> Vec<Edit> {
        BlankLineAfterNamespaceFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            indent: IndentStyle::default(),
            options: Default::default(),
        })
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\n\nnamespace App;\n\nuse Foo;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_missing_blank_line() {
        let source = "<?php\n\nnamespace App;\nuse Foo;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "\n");
    }

    #[test]
    fn test_too_many_blank_lines() {
        let source = "<?php\n\nnamespace App;\n\n\n\nuse Foo;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_namespace_with_braces() {
        // Namespace with braces doesn't need blank line
        let source = "<?php\n\nnamespace App {\n    class Foo {}\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_end_of_file() {
        let source = "<?php\n\nnamespace App;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
