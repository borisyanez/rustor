//! Remove leading slash from use imports

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes leading backslash from use imports
pub struct NoLeadingImportSlashFixer;

impl Fixer for NoLeadingImportSlashFixer {
    fn name(&self) -> &'static str {
        "no_leading_import_slash"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_leading_import_slash"
    }

    fn description(&self) -> &'static str {
        "Remove leading backslash from use imports"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: use \Namespace\Class;
        let re = Regex::new(r"(?m)^(\s*use\s+)\\([A-Z])").unwrap();

        for cap in re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let prefix = cap.get(1).unwrap();
            let first_char = cap.get(2).unwrap();

            // Check not in string/comment
            if is_in_string_or_comment(&source[..full_match.start()]) {
                continue;
            }

            let replacement = format!("{}{}", prefix.as_str(), first_char.as_str());

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                "Remove leading backslash from import".to_string(),
                "no_leading_import_slash",
            ));
        }

        edits
    }
}

fn is_in_string_or_comment(before: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        if !in_single_quote && !in_double_quote && !in_block_comment {
            if c == '/' && prev_char == '/' {
                in_line_comment = true;
            }
            if c == '#' {
                in_line_comment = true;
            }
        }

        if !in_single_quote && !in_double_quote && !in_line_comment {
            if c == '*' && prev_char == '/' {
                in_block_comment = true;
            }
            if c == '/' && prev_char == '*' && in_block_comment {
                in_block_comment = false;
            }
        }

        if c == '\n' {
            in_line_comment = false;
        }

        if !in_line_comment && !in_block_comment {
            if c == '\'' && prev_char != '\\' && !in_double_quote {
                in_single_quote = !in_single_quote;
            }
            if c == '"' && prev_char != '\\' && !in_single_quote {
                in_double_quote = !in_double_quote;
            }
        }

        prev_char = c;
    }

    in_single_quote || in_double_quote || in_line_comment || in_block_comment
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        NoLeadingImportSlashFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let edits = check("<?php\n\nuse App\\Service;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_leading_slash() {
        let source = "<?php\n\nuse \\App\\Service;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(!edits[0].replacement.contains("\\\\"));
    }

    #[test]
    fn test_multiple_imports() {
        let source = "<?php\n\nuse \\App\\Service;\nuse \\App\\Model;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_indented_import() {
        let source = "<?php\n\n    use \\App\\Service;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'use \\\\App\\\\Service';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
