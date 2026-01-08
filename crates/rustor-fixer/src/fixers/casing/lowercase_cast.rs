//! Lowercase cast fixer
//!
//! Ensures cast operators are lowercase.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures cast operators are lowercase
pub struct LowercaseCastFixer;

impl Fixer for LowercaseCastFixer {
    fn name(&self) -> &'static str {
        "lowercase_cast"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "lowercase_cast"
    }

    fn description(&self) -> &'static str {
        "Ensure cast operators are lowercase"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match uppercase or mixed case casts
        // PHP casts: (int), (integer), (bool), (boolean), (float), (double), (real),
        //            (string), (array), (object), (unset), (binary)
        let re = Regex::new(r"\(\s*(INT|INTEGER|BOOL|BOOLEAN|FLOAT|DOUBLE|REAL|STRING|ARRAY|OBJECT|UNSET|BINARY|Int|Integer|Bool|Boolean|Float|Double|Real|String|Array|Object|Unset|Binary)\s*\)").unwrap();

        for cap in re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let cast_type = cap.get(1).unwrap();

            // Skip if in string or comment
            if is_in_string_or_comment(&source[..full_match.start()]) {
                continue;
            }

            let lowercase = cast_type.as_str().to_lowercase();
            if cast_type.as_str() != lowercase {
                // Preserve spacing
                let match_str = full_match.as_str();
                let new_cast = match_str.replace(cast_type.as_str(), &lowercase);

                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    new_cast,
                    format!("Use lowercase cast ({})", lowercase),
                    "lowercase_cast",
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
        LowercaseCastFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\n$a = (int)$b;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_uppercase_int() {
        let source = "<?php\n$a = (INT)$b;\n";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "(int)");
    }

    #[test]
    fn test_uppercase_string() {
        let source = "<?php\n$a = (STRING)$b;\n";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "(string)");
    }

    #[test]
    fn test_mixed_case() {
        let source = "<?php\n$a = (Bool)$b;\n";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "(bool)");
    }

    #[test]
    fn test_with_spaces() {
        let source = "<?php\n$a = ( INT )$b;\n";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "( int )");
    }

    #[test]
    fn test_multiple_casts() {
        let source = "<?php\n$a = (INT)$b + (FLOAT)$c;\n";
        let edits = check(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = '(INT)value';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_all_cast_types() {
        let source = "<?php\n$a = (INTEGER)$b; $b = (BOOLEAN)$c; $c = (DOUBLE)$d; $d = (ARRAY)$e; $e = (OBJECT)$f;\n";
        let edits = check(source);
        assert_eq!(edits.len(), 5);
    }
}
