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

        // Binary operators that need spaces
        let operators = [
            // Assignment
            ("=", "="),
            ("+=", "\\+="),
            ("-=", "-="),
            ("*=", "\\*="),
            ("/=", "/="),
            ("%=", "%="),
            (".=", "\\.="),
            ("??=", "\\?\\?="),
            // Comparison
            ("==", "=="),
            ("===", "==="),
            ("!=", "!="),
            ("!==", "!=="),
            ("<>", "<>"),
            ("<", "<"),
            (">", ">"),
            ("<=", "<="),
            (">=", ">="),
            ("<=>", "<=>"),
            // Logical
            ("&&", "&&"),
            ("||", "\\|\\|"),
            ("??", "\\?\\?"),
            // Arithmetic (be careful with negative numbers)
            ("+", "\\+"),
            ("-", "-"),
            ("*", "\\*"),
            ("/", "/"),
            ("%", "%"),
            ("**", "\\*\\*"),
            // Bitwise
            ("&", "&"),
            ("|", "\\|"),
            ("^", "\\^"),
            ("<<", "<<"),
            (">>", ">>"),
        ];

        for (op, pattern) in &operators {
            // Skip single character operators for now - too many edge cases
            if op.len() == 1 && !["="].contains(op) {
                continue;
            }

            // Pattern: operator without proper spacing
            // Match operator with missing space before or after
            let first_char_escaped = regex::escape(&op[..1]);
            let re_str = format!(
                r"([^\s{first}]){pat}|{pat}([^\s=])",
                first = first_char_escaped,
                pat = pattern,
            );

            if let Ok(re) = Regex::new(&re_str) {
                for mat in re.find_iter(source) {
                    // Skip if in string/comment
                    if is_in_string_or_comment(&source[..mat.start()]) {
                        continue;
                    }

                    // Check for specific edge cases
                    let context_start = mat.start().saturating_sub(3);
                    let context = &source[context_start..mat.end().min(source.len())];

                    // Skip if part of arrow function =>
                    if context.contains("=>") && *op == "=" {
                        continue;
                    }

                    // Skip negative numbers
                    if *op == "-" {
                        let before = &source[..mat.start()];
                        if before.trim_end().ends_with(&['=', '(', ',', '[', ':', '?'][..]) {
                            continue;
                        }
                    }

                    // Skip type hints (e.g., ?string)
                    if (*op == "?" || op.starts_with('?')) && context.contains("?") {
                        let after_pos = mat.end();
                        if after_pos < source.len() {
                            let after_char = source[after_pos..].chars().next();
                            if after_char.map(|c| c.is_alphabetic()).unwrap_or(false) {
                                continue;
                            }
                        }
                    }
                }
            }
        }

        // Focus on the most important: assignment operator
        // Match = that doesn't have space on both sides
        let assign_re = Regex::new(r"[^\s=!<>+\-*/%&|^.?]=|=[^\s=>]").unwrap();

        for mat in assign_re.find_iter(source) {
            if is_in_string_or_comment(&source[..mat.start()]) {
                continue;
            }

            let matched = mat.as_str();

            // Skip ==, ===, !=, !==, <=, >=, <>, =>, +=, -=, etc.
            if matched.contains("==") || matched.contains("=>") || matched.contains("!=")
                || matched.contains("<=") || matched.contains(">=") || matched.contains("<>")
                || matched.ends_with("+=") || matched.ends_with("-=")
                || matched.ends_with("*=") || matched.ends_with("/=")
                || matched.ends_with(".=") || matched.ends_with("%=")
                || matched.ends_with("&=") || matched.ends_with("|=")
                || matched.ends_with("^=") || matched.ends_with("??=")
            {
                continue;
            }

            // Find the actual = position
            let eq_pos = matched.find('=').unwrap();
            let abs_pos = mat.start() + eq_pos;

            // Get surrounding context
            let before_char = if abs_pos > 0 {
                source[..abs_pos].chars().last()
            } else {
                None
            };
            let after_char = if abs_pos + 1 < source.len() {
                source[abs_pos + 1..].chars().next()
            } else {
                None
            };

            let needs_space_before = before_char.map(|c| c != ' ' && c != '\t').unwrap_or(false);
            let needs_space_after = after_char.map(|c| c != ' ' && c != '\t' && c != '=').unwrap_or(false);

            if needs_space_before || needs_space_after {
                let start = if needs_space_before { abs_pos } else { abs_pos };
                let end = if needs_space_after { abs_pos + 1 } else { abs_pos + 1 };

                let replacement = if needs_space_before && needs_space_after {
                    " = ".to_string()
                } else if needs_space_before {
                    " =".to_string()
                } else {
                    "= ".to_string()
                };

                edits.push(edit_with_rule(
                    start,
                    end,
                    replacement,
                    "Add space around assignment operator".to_string(),
                    "binary_operator_spaces",
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
}
