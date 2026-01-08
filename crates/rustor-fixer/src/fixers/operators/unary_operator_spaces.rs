//! Configure spacing around unary operators

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures no space after unary operators (!, ++, --, ~)
pub struct UnaryOperatorSpacesFixer;

impl Fixer for UnaryOperatorSpacesFixer {
    fn name(&self) -> &'static str {
        "unary_operator_spaces"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "unary_operator_spaces"
    }

    fn description(&self) -> &'static str {
        "Remove space after unary operators"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match ! followed by space(s) then variable/expression
        // !  $var  ->  !$var
        let not_re = Regex::new(r"!\s+(\$|[a-zA-Z_\(])").unwrap();

        for cap in not_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();

            if is_in_string_or_comment(&source[..full_match.start()]) {
                continue;
            }

            // Check it's not !== or !=
            if full_match.start() > 0 {
                let before = &source[..full_match.start()];
                if before.ends_with('!') {
                    continue;
                }
            }

            let next_char = cap.get(1).unwrap().as_str();
            let replacement = format!("!{}", next_char);

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                "Remove space after logical not operator".to_string(),
                "unary_operator_spaces",
            ));
        }

        // Match ~ followed by space(s) then variable/expression
        let tilde_re = Regex::new(r"~\s+(\$|[a-zA-Z_\(0-9])").unwrap();

        for cap in tilde_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();

            if is_in_string_or_comment(&source[..full_match.start()]) {
                continue;
            }

            let next_char = cap.get(1).unwrap().as_str();
            let replacement = format!("~{}", next_char);

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                "Remove space after bitwise not operator".to_string(),
                "unary_operator_spaces",
            ));
        }

        // Note: ++/-- are tricky because they can be prefix or postfix
        // We handle prefix case: ++ $var -> ++$var
        let inc_re = Regex::new(r"\+\+\s+\$").unwrap();

        for mat in inc_re.find_iter(source) {
            if is_in_string_or_comment(&source[..mat.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                mat.start(),
                mat.end(),
                "++$".to_string(),
                "Remove space after increment operator".to_string(),
                "unary_operator_spaces",
            ));
        }

        let dec_re = Regex::new(r"--\s+\$").unwrap();

        for mat in dec_re.find_iter(source) {
            if is_in_string_or_comment(&source[..mat.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                mat.start(),
                mat.end(),
                "--$".to_string(),
                "Remove space after decrement operator".to_string(),
                "unary_operator_spaces",
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
        UnaryOperatorSpacesFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let edits = check("<?php\nif (!$var) { }\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_space_after_not() {
        let source = "<?php\nif (! $var) { }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.starts_with('!'));
        assert!(!edits[0].replacement.contains(' '));
    }

    #[test]
    fn test_space_after_tilde() {
        let source = "<?php\n$a = ~ $b;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_space_after_increment() {
        let source = "<?php\n++ $i;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "++$");
    }

    #[test]
    fn test_space_after_decrement() {
        let source = "<?php\n-- $i;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "--$");
    }

    #[test]
    fn test_not_equal_unchanged() {
        let edits = check("<?php\nif ($a != $b) { }\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_not_identical_unchanged() {
        let edits = check("<?php\nif ($a !== $b) { }\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let edits = check("<?php\n$a = '! test';\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_function_call() {
        let source = "<?php\nif (! empty($arr)) { }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }
}
