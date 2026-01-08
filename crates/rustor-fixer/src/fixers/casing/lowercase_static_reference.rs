//! Lowercase static references (self, static, parent)

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures self, static, parent are lowercase
pub struct LowercaseStaticReferenceFixer;

impl Fixer for LowercaseStaticReferenceFixer {
    fn name(&self) -> &'static str {
        "lowercase_static_reference"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "lowercase_static_reference"
    }

    fn description(&self) -> &'static str {
        "Class static references self, static, parent must be lowercase"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        let keywords = ["self", "static", "parent"];

        for keyword in &keywords {
            // Match keyword followed by :: (static method/property access)
            // We need to check what follows manually since regex crate doesn't support look-ahead
            let pattern = format!(r"(?i)\b({})\b", keyword);
            let re = Regex::new(&pattern).unwrap();

            for cap in re.captures_iter(source) {
                let matched = cap.get(1).unwrap();
                let matched_str = matched.as_str();

                // Check what follows the keyword
                let after_match = &source[matched.end()..];
                let followed_by_double_colon = after_match.trim_start().starts_with("::");

                // Skip if not followed by ::
                if !followed_by_double_colon {
                    continue;
                }

                // Skip if already lowercase
                if matched_str == *keyword {
                    continue;
                }

                // Skip if in string or comment
                if is_in_string_or_comment(&source[..matched.start()]) {
                    continue;
                }

                edits.push(edit_with_rule(
                    matched.start(),
                    matched.end(),
                    keyword.to_string(),
                    format!("Lowercase '{}'", matched_str),
                    "lowercase_static_reference",
                ));
            }
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
        LowercaseStaticReferenceFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_lowercase_unchanged() {
        let edits = check("<?php\nself::method();\nstatic::$prop;\nparent::__construct();\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_uppercase_self() {
        let source = "<?php\nSELF::method();\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "self");
    }

    #[test]
    fn test_uppercase_static() {
        let source = "<?php\nSTATIC::$property;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "static");
    }

    #[test]
    fn test_uppercase_parent() {
        let source = "<?php\nPARENT::__construct();\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "parent");
    }

    #[test]
    fn test_mixed_case() {
        let source = "<?php\nSelf::method();\nStatic::$prop;\nParent::test();\n";
        let edits = check(source);

        assert_eq!(edits.len(), 3);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'SELF::method';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_class_context() {
        let source = "<?php\nclass Foo {\n    public function test() {\n        return SELF::class;\n    }\n}\n";
        let edits = check(source);

        assert!(!edits.is_empty());
    }
}
