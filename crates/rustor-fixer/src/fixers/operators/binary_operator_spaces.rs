//! Configure spacing around binary operators

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures single space around binary operators
pub struct BinaryOperatorSpacesFixer;

impl Fixer for BinaryOperatorSpacesFixer {
    fn name(&self) -> &'static str {
        "binary_operator_spaces"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "binary_operator_spaces"
    }

    fn description(&self) -> &'static str {
        "Ensure single space around binary operators"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Process character by character to find standalone = that need spacing
        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            // Calculate byte position for this character
            let byte_pos: usize = chars[..i].iter().map(|c| c.len_utf8()).sum();

            // Skip strings and comments
            if is_in_string_or_comment(&source[..byte_pos]) {
                i += 1;
                continue;
            }

            // Skip = inside declare() statements - they shouldn't have spaces per PSR-12
            if chars[i] == '=' && is_in_declare(&source[..byte_pos]) {
                i += 1;
                continue;
            }

            if chars[i] == '=' {
                // Check what's before and after to determine the operator type
                let prev = if i > 0 { Some(chars[i - 1]) } else { None };
                let next = if i + 1 < len { Some(chars[i + 1]) } else { None };
                let _next2 = if i + 2 < len { Some(chars[i + 2]) } else { None };

                // Skip compound operators: ===, ==, !==, !=, <=, >=, <>, =>, +=, -=, *=, /=, .=, %=, &=, |=, ^=, ??=
                let is_compound = match (prev, next) {
                    // === or ==
                    (Some('='), _) => true,
                    (_, Some('=')) => true,
                    // !==, !=
                    (Some('!'), _) => true,
                    // <=, >=, <>
                    (Some('<'), _) => true,
                    (Some('>'), _) => true,
                    // =>
                    (_, Some('>')) => true,
                    // +=, -=, *=, /=, .=, %=, &=, |=, ^=
                    (Some('+'), _) => true,
                    (Some('-'), _) => true,
                    (Some('*'), _) => true,
                    (Some('/'), _) => true,
                    (Some('.'), _) => true,
                    (Some('%'), _) => true,
                    (Some('&'), _) => true,
                    (Some('|'), _) => true,
                    (Some('^'), _) => true,
                    // ??=
                    (Some('?'), _) => true,
                    _ => false,
                };

                if is_compound {
                    i += 1;
                    continue;
                }

                // This is a standalone = (assignment)
                // Check if it needs spacing
                let needs_space_before = prev.map(|c| !c.is_whitespace()).unwrap_or(false);
                let needs_space_after = next.map(|c| !c.is_whitespace() && c != '=' && c != '>').unwrap_or(false);

                if needs_space_before || needs_space_after {
                    let replacement = if needs_space_before && needs_space_after {
                        " = ".to_string()
                    } else if needs_space_before {
                        " =".to_string()
                    } else {
                        "= ".to_string()
                    };

                    // Calculate byte position
                    let byte_pos: usize = chars[..i].iter().map(|c| c.len_utf8()).sum();

                    edits.push(edit_with_rule(
                        byte_pos,
                        byte_pos + 1,
                        replacement,
                        "Add space around assignment operator".to_string(),
                        "binary_operator_spaces",
                    ));
                }
            }

            i += 1;
        }

        // Handle && and || operators
        let mut i = 0;
        while i < len.saturating_sub(1) {
            let byte_pos: usize = chars[..i].iter().map(|c| c.len_utf8()).sum();

            // Skip strings and comments
            if is_in_string_or_comment(&source[..byte_pos]) {
                i += 1;
                continue;
            }

            let curr = chars[i];
            let next = chars[i + 1];

            // Check for && or ||
            if (curr == '&' && next == '&') || (curr == '|' && next == '|') {
                let op = if curr == '&' { "&&" } else { "||" };
                let prev = if i > 0 { Some(chars[i - 1]) } else { None };
                let after = if i + 2 < len { Some(chars[i + 2]) } else { None };

                let needs_space_before = prev.map(|c| !c.is_whitespace()).unwrap_or(false);
                let needs_space_after = after.map(|c| !c.is_whitespace()).unwrap_or(false);

                if needs_space_before || needs_space_after {
                    let replacement = if needs_space_before && needs_space_after {
                        format!(" {} ", op)
                    } else if needs_space_before {
                        format!(" {}", op)
                    } else {
                        format!("{} ", op)
                    };

                    edits.push(edit_with_rule(
                        byte_pos,
                        byte_pos + 2,
                        replacement,
                        format!("Add space around {} operator", op),
                        "binary_operator_spaces",
                    ));

                    i += 2; // Skip both characters
                    continue;
                }
            }

            i += 1;
        }

        edits
    }
}

/// Check if we're inside a declare() statement (between 'declare(' and ')')
fn is_in_declare(before: &str) -> bool {
    // Look for the last declare( and check if we've seen the closing )
    let lower = before.to_lowercase();
    if let Some(declare_pos) = lower.rfind("declare(") {
        let after_declare = &before[declare_pos..];
        // Count parentheses to handle nested expressions
        let mut depth = 0;
        for c in after_declare.chars() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    return false; // We've closed the declare()
                }
            }
        }
        // Still inside declare() if depth > 0
        return depth > 0;
    }
    false
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
        BinaryOperatorSpacesFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let edits = check("<?php\n$a = 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_no_space_before() {
        let source = "<?php\n$a= 1;\n";
        let edits = check(source);

        assert!(!edits.is_empty());
    }

    #[test]
    fn test_no_space_after() {
        let source = "<?php\n$a =1;\n";
        let edits = check(source);

        assert!(!edits.is_empty());
    }

    #[test]
    fn test_comparison_unchanged() {
        let edits = check("<?php\nif ($a == $b) { }\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_arrow_function_unchanged() {
        let edits = check("<?php\n$fn = fn($x) => $x * 2;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_compound_assignment_unchanged() {
        let edits = check("<?php\n$a += 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let edits = check("<?php\n$a = 'b=c';\n");
        // Should only check code, not string content
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_declare_statement() {
        // PSR-12 says no spaces in declare statements
        let edits = check("<?php\ndeclare(strict_types=1);\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_triple_equals_unchanged() {
        let edits = check("<?php\nif ($a === $b) { }\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_not_equals_unchanged() {
        let edits = check("<?php\nif ($a !== $b) { }\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_arrow_unchanged() {
        let edits = check("<?php\nforeach ($arr as $k => $v) { }\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_and_operator_no_space() {
        let source = "<?php\n$a&&$b;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" && "));
    }

    #[test]
    fn test_or_operator_no_space() {
        let source = "<?php\n$a||$b;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" || "));
    }

    #[test]
    fn test_and_operator_correct() {
        let edits = check("<?php\n$a && $b;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_or_operator_correct() {
        let edits = check("<?php\n$a || $b;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_and_operator_space_after_only() {
        let source = "<?php\n$a&& $b;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, " &&");
    }

    #[test]
    fn test_and_operator_space_before_only() {
        let source = "<?php\n$a &&$b;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "&& ");
    }
}
