//! Convert `else if` to `elseif`

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Converts `else if` to `elseif` for PSR-12 compliance
pub struct ElseifFixer;

impl Fixer for ElseifFixer {
    fn name(&self) -> &'static str {
        "elseif"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "elseif"
    }

    fn description(&self) -> &'static str {
        "Convert else if to elseif"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match "else if" with various whitespace patterns
        // Must not be in string or comment
        let re = Regex::new(r"(?i)\belse\s+if\b").unwrap();

        for mat in re.find_iter(source) {
            // Check if in string or comment
            if is_in_string_or_comment(&source[..mat.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                mat.start(),
                mat.end(),
                "elseif".to_string(),
                "Convert 'else if' to 'elseif'".to_string(),
                "elseif",
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
        ElseifFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_elseif_unchanged() {
        let edits = check("<?php\nif ($a) { } elseif ($b) { }\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_else_if_to_elseif() {
        let source = "<?php\nif ($a) { } else if ($b) { }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "elseif");
    }

    #[test]
    fn test_else_if_multiline() {
        let source = "<?php\nif ($a) {\n} else\nif ($b) {\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_else_if() {
        let source = "<?php\nif ($a) { } else if ($b) { } else if ($c) { }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'else if';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_comment() {
        let source = "<?php\n// else if\nif ($a) { }\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
