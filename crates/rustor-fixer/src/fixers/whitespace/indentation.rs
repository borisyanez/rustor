//! Normalize indentation style

use rustor_core::Edit;
use crate::config::IndentStyle;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Normalizes indentation to spaces or tabs
pub struct IndentationFixer;

impl Fixer for IndentationFixer {
    fn name(&self) -> &'static str {
        "indentation"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "indentation_type"
    }

    fn description(&self) -> &'static str {
        "Normalize indentation to spaces or tabs"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let mut offset = 0;

        for (line_num, line) in source.lines().enumerate() {
            // Find leading whitespace
            let leading: String = line.chars().take_while(|c| c.is_whitespace()).collect();

            if !leading.is_empty() {
                let normalized = normalize_indent(&leading, &config.indent);

                if normalized != leading {
                    edits.push(edit_with_rule(
                        offset,
                        offset + leading.len(),
                        normalized,
                        format!("Normalize indentation on line {}", line_num + 1),
                        "indentation_type",
                    ));
                }
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

/// Normalize indentation string to target style
fn normalize_indent(indent: &str, style: &IndentStyle) -> String {
    match style {
        IndentStyle::Spaces(size) => {
            // Convert tabs to spaces
            let mut result = String::new();
            for c in indent.chars() {
                if c == '\t' {
                    result.push_str(&" ".repeat(*size));
                } else {
                    result.push(c);
                }
            }
            result
        }
        IndentStyle::Tabs => {
            // Convert spaces to tabs (default 4 spaces = 1 tab)
            let space_count = indent.chars().filter(|c| *c == ' ').count();
            let tab_count = indent.chars().filter(|c| *c == '\t').count();

            // Calculate total indentation level
            let total_spaces = space_count + tab_count * 4;
            let levels = total_spaces / 4;
            let remainder = total_spaces % 4;

            let mut result = "\t".repeat(levels);
            if remainder > 0 {
                result.push_str(&" ".repeat(remainder));
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_spaces(source: &str, size: usize) -> Vec<Edit> {
        IndentationFixer.check(source, &FixerConfig {
            indent: IndentStyle::Spaces(size),
            line_ending: crate::config::LineEnding::Lf,
            options: Default::default(),
        })
    }

    fn check_tabs(source: &str) -> Vec<Edit> {
        IndentationFixer.check(source, &FixerConfig {
            indent: IndentStyle::Tabs,
            line_ending: crate::config::LineEnding::Lf,
            options: Default::default(),
        })
    }

    #[test]
    fn test_spaces_no_change() {
        let source = "<?php\n    $a = 1;\n";
        let edits = check_spaces(source, 4);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_tabs_to_spaces() {
        let source = "<?php\n\t$a = 1;\n";
        let edits = check_spaces(source, 4);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "    ");
    }

    #[test]
    fn test_tabs_to_2_spaces() {
        let source = "<?php\n\t$a = 1;\n";
        let edits = check_spaces(source, 2);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "  ");
    }

    #[test]
    fn test_spaces_to_tabs() {
        let source = "<?php\n    $a = 1;\n";
        let edits = check_tabs(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "\t");
    }

    #[test]
    fn test_mixed_to_spaces() {
        let source = "<?php\n\t  $a = 1;\n"; // tab + 2 spaces = 6 spaces
        let edits = check_spaces(source, 4);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "      "); // 6 spaces
    }

    #[test]
    fn test_nested_indentation() {
        let source = "<?php\n\t\t$a = 1;\n";
        let edits = check_spaces(source, 4);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "        "); // 8 spaces
    }

    #[test]
    fn test_tabs_no_change() {
        let source = "<?php\n\t$a = 1;\n";
        let edits = check_tabs(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_normalize_indent_spaces() {
        assert_eq!(normalize_indent("\t", &IndentStyle::Spaces(4)), "    ");
        assert_eq!(normalize_indent("\t\t", &IndentStyle::Spaces(4)), "        ");
        assert_eq!(normalize_indent("    ", &IndentStyle::Spaces(4)), "    ");
        assert_eq!(normalize_indent("\t  ", &IndentStyle::Spaces(4)), "      ");
    }

    #[test]
    fn test_normalize_indent_tabs() {
        assert_eq!(normalize_indent("    ", &IndentStyle::Tabs), "\t");
        assert_eq!(normalize_indent("        ", &IndentStyle::Tabs), "\t\t");
        assert_eq!(normalize_indent("\t", &IndentStyle::Tabs), "\t");
        assert_eq!(normalize_indent("  ", &IndentStyle::Tabs), "  "); // Less than 4 spaces
    }
}
