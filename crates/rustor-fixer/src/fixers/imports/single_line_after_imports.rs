//! Ensure single blank line after use imports

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures a blank line after the last use import before class/function
pub struct SingleLineAfterImportsFixer;

impl Fixer for SingleLineAfterImportsFixer {
    fn name(&self) -> &'static str {
        "single_line_after_imports"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "single_line_after_imports"
    }

    fn description(&self) -> &'static str {
        "Ensure single blank line after use imports"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Find all use statements
        // Use [ \t]* instead of \s* to avoid matching newlines
        let use_re = Regex::new(r"(?m)^use\s+[^;]+;[ \t]*$").unwrap();
        let mut last_use_end: Option<usize> = None;

        for mat in use_re.find_iter(source) {
            last_use_end = Some(mat.end());
        }

        let Some(use_end) = last_use_end else {
            return edits;
        };

        // Find the position after the use statement's newline
        let mut pos = use_end;
        while pos < source.len() && (source.as_bytes()[pos] == b'\n' || source.as_bytes()[pos] == b'\r') {
            pos += 1;
        }

        if pos >= source.len() {
            return edits;
        }

        // Check what follows
        let after = &source[pos..];
        let next_line = after.lines().next().unwrap_or("");
        let trimmed = next_line.trim();

        // If followed by class, interface, trait, function, or const - need blank line
        let needs_blank = trimmed.starts_with("class ")
            || trimmed.starts_with("abstract class")
            || trimmed.starts_with("final class")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("trait ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("#[")
            || trimmed.starts_with("/**");

        if !needs_blank {
            return edits;
        }

        // Count blank lines between use and next content
        let between = &source[use_end..pos];
        let newline_count = between.chars().filter(|c| *c == '\n').count();

        if newline_count < 2 {
            // Need to add blank line
            edits.push(edit_with_rule(
                use_end,
                pos,
                format!("{}{}", line_ending, line_ending),
                "Add blank line after imports".to_string(),
                "single_line_after_imports",
            ));
        } else if newline_count > 2 {
            // Too many blank lines
            edits.push(edit_with_rule(
                use_end,
                pos,
                format!("{}{}", line_ending, line_ending),
                "Ensure single blank line after imports".to_string(),
                "single_line_after_imports",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IndentStyle, LineEnding};

    fn check(source: &str) -> Vec<Edit> {
        SingleLineAfterImportsFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            indent: IndentStyle::default(),
            options: Default::default(),
        })
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\n\nnamespace App;\n\nuse Foo;\n\nclass Bar {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_missing_blank_line_before_class() {
        let source = "<?php\n\nnamespace App;\n\nuse Foo;\nclass Bar {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_too_many_blank_lines() {
        let source = "<?php\n\nnamespace App;\n\nuse Foo;\n\n\n\nclass Bar {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_no_imports() {
        let source = "<?php\n\nnamespace App;\n\nclass Bar {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_before_interface() {
        let source = "<?php\n\nuse Foo;\ninterface Bar {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_before_attribute() {
        let source = "<?php\n\nuse Foo;\n#[Attribute]\nclass Bar {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }
}
