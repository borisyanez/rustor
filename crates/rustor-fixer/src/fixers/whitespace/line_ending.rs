//! Normalize line endings

use rustor_core::Edit;
use crate::config::LineEnding;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Normalizes line endings to LF or CRLF
pub struct LineEndingFixer;

impl Fixer for LineEndingFixer {
    fn name(&self) -> &'static str {
        "line_ending"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "line_ending"
    }

    fn description(&self) -> &'static str {
        "Normalize line endings (LF for Unix, CRLF for Windows)"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let target_ending = config.line_ending.as_str();

        let mut i = 0;
        let bytes = source.as_bytes();

        while i < bytes.len() {
            if bytes[i] == b'\r' {
                // Found CR
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    // CRLF
                    if config.line_ending == LineEnding::Lf {
                        // Convert CRLF to LF
                        edits.push(edit_with_rule(
                            i,
                            i + 2,
                            "\n".to_string(),
                            "Convert CRLF to LF".to_string(),
                            "line_ending",
                        ));
                    }
                    i += 2;
                } else {
                    // Bare CR (old Mac style)
                    edits.push(edit_with_rule(
                        i,
                        i + 1,
                        target_ending.to_string(),
                        format!("Convert CR to {}", if config.line_ending == LineEnding::Lf { "LF" } else { "CRLF" }),
                        "line_ending",
                    ));
                    i += 1;
                }
            } else if bytes[i] == b'\n' {
                // LF
                if config.line_ending == LineEnding::CrLf {
                    // Convert LF to CRLF
                    edits.push(edit_with_rule(
                        i,
                        i + 1,
                        "\r\n".to_string(),
                        "Convert LF to CRLF".to_string(),
                        "line_ending",
                    ));
                }
                i += 1;
            } else {
                i += 1;
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IndentStyle;

    fn check_lf(source: &str) -> Vec<Edit> {
        LineEndingFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            indent: IndentStyle::default(),
            options: Default::default(),
        })
    }

    fn check_crlf(source: &str) -> Vec<Edit> {
        LineEndingFixer.check(source, &FixerConfig {
            line_ending: LineEnding::CrLf,
            indent: IndentStyle::default(),
            options: Default::default(),
        })
    }

    #[test]
    fn test_lf_no_change() {
        let edits = check_lf("<?php\n$a = 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_crlf_to_lf() {
        let source = "<?php\r\n$a = 1;\r\n";
        let edits = check_lf(source);

        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].replacement, "\n");
        assert_eq!(edits[1].replacement, "\n");
    }

    #[test]
    fn test_lf_to_crlf() {
        let source = "<?php\n$a = 1;\n";
        let edits = check_crlf(source);

        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].replacement, "\r\n");
        assert_eq!(edits[1].replacement, "\r\n");
    }

    #[test]
    fn test_bare_cr() {
        let source = "<?php\r$a = 1;\r";
        let edits = check_lf(source);

        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].replacement, "\n");
    }

    #[test]
    fn test_mixed_endings() {
        let source = "<?php\n$a = 1;\r\n$b = 2;\r$c = 3;\n";
        let edits = check_lf(source);

        // Should convert CRLF and CR to LF
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_crlf_no_change() {
        let source = "<?php\r\n$a = 1;\r\n";
        let edits = check_crlf(source);
        assert!(edits.is_empty());
    }
}
