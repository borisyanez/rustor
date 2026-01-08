//! Remove blank lines after class opening brace

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes blank lines immediately after class, interface, or trait opening brace
pub struct NoBlankLinesAfterClassOpeningFixer;

impl Fixer for NoBlankLinesAfterClassOpeningFixer {
    fn name(&self) -> &'static str {
        "no_blank_lines_after_class_opening"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_blank_lines_after_class_opening"
    }

    fn description(&self) -> &'static str {
        "Remove blank lines after class opening brace"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Match class/interface/trait followed by { and then blank lines
        // This regex finds the opening brace of class definitions
        let class_re = Regex::new(r"(?m)\b(class|interface|trait)\s+\w+[^{]*\{[ \t]*\n").unwrap();

        for mat in class_re.find_iter(source) {
            let brace_end = mat.end();

            if brace_end >= source.len() {
                continue;
            }

            let after = &source[brace_end..];

            // Count leading blank lines after the opening brace
            let mut blank_line_end = 0;
            let mut newline_count = 0;

            for (i, c) in after.chars().enumerate() {
                if c == '\n' {
                    newline_count += 1;
                    blank_line_end = i + 1;
                } else if c == '\r' {
                    // Skip \r in \r\n
                    continue;
                } else if c == ' ' || c == '\t' {
                    // Skip whitespace on blank lines
                    continue;
                } else {
                    // Found non-whitespace
                    break;
                }
            }

            // If there are blank lines (more than 0 newlines after the brace line)
            if newline_count > 0 {
                edits.push(edit_with_rule(
                    brace_end,
                    brace_end + blank_line_end,
                    line_ending.to_string(),
                    "Remove blank lines after class opening".to_string(),
                    "no_blank_lines_after_class_opening",
                ));
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LineEnding;

    fn check(source: &str) -> Vec<Edit> {
        NoBlankLinesAfterClassOpeningFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        })
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nclass Foo {\n    public $bar;\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_blank_line_after_class() {
        let source = "<?php\nclass Foo {\n\n    public $bar;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_blank_lines() {
        let source = "<?php\nclass Foo {\n\n\n\n    public $bar;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_interface() {
        let source = "<?php\ninterface Foo {\n\n    public function bar();\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_trait() {
        let source = "<?php\ntrait Foo {\n\n    public function bar() {}\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_with_extends() {
        let source = "<?php\nclass Foo extends Bar {\n\n    public $baz;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }
}
