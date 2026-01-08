//! Enforce single line comment style (// over #)

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Converts # comments to // comments
pub struct SingleLineCommentStyleFixer;

impl Fixer for SingleLineCommentStyleFixer {
    fn name(&self) -> &'static str {
        "single_line_comment_style"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "single_line_comment_style"
    }

    fn description(&self) -> &'static str {
        "Convert # comments to // style"
    }

    fn priority(&self) -> i32 {
        60
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match # comments at start of line (with optional leading whitespace)
        let re = Regex::new(r"(?m)^([ \t]*)#").unwrap();

        for cap in re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let indent = cap.get(1).unwrap().as_str();

            // Skip #[ which is attribute syntax in PHP 8
            let after_match = &source[full_match.end()..];
            if after_match.starts_with('[') {
                continue;
            }

            // Check if we're in a string by looking at context
            if is_in_string_or_heredoc(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{}//", indent),
                "Convert # comment to // style".to_string(),
                "single_line_comment_style",
            ));
        }

        edits
    }
}

fn is_in_string_or_heredoc(before: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_heredoc = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        if !in_heredoc {
            if c == '\'' && prev_char != '\\' && !in_double_quote {
                in_single_quote = !in_single_quote;
            }
            if c == '"' && prev_char != '\\' && !in_single_quote {
                in_double_quote = !in_double_quote;
            }
        }

        // Very basic heredoc detection - look for <<<
        if c == '<' && prev_char == '<' && !in_single_quote && !in_double_quote {
            // This is a simplification - real heredoc detection is complex
            in_heredoc = true;
        }

        prev_char = c;
    }

    in_single_quote || in_double_quote || in_heredoc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        SingleLineCommentStyleFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\n// comment\n$a = 1;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_hash_comment() {
        let source = "<?php\n# comment\n$a = 1;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "//");
    }

    #[test]
    fn test_indented_hash_comment() {
        let source = "<?php\n    # comment\n$a = 1;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "    //");
    }

    #[test]
    fn test_multiple_hash_comments() {
        let source = "<?php\n# first\n# second\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_attribute() {
        // PHP 8 attribute syntax
        let source = "<?php\n#[Attribute]\nclass Foo {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = '# not a comment';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
