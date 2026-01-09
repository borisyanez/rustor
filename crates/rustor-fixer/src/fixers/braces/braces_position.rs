//! Fix opening brace position

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, FixerOption, OptionType, ConfigValue, edit_with_rule};

/// Controls placement of opening braces for classes, methods, control structures
pub struct BracesPositionFixer;

impl Fixer for BracesPositionFixer {
    fn name(&self) -> &'static str {
        "braces_position"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "braces_position"
    }

    fn description(&self) -> &'static str {
        "Fix opening brace position (same line or next line)"
    }

    fn priority(&self) -> i32 {
        35
    }

    fn options(&self) -> Vec<FixerOption> {
        vec![
            FixerOption {
                name: "control_structures_opening_brace",
                description: "Position for control structure braces: same_line, next_line_unless_newline_at_signature_end",
                option_type: OptionType::Enum(vec!["same_line", "next_line_unless_newline_at_signature_end"]),
                default: Some(ConfigValue::String("same_line".to_string())),
            },
            FixerOption {
                name: "functions_opening_brace",
                description: "Position for function/method braces",
                option_type: OptionType::Enum(vec!["same_line", "next_line_unless_newline_at_signature_end"]),
                default: Some(ConfigValue::String("next_line_unless_newline_at_signature_end".to_string())),
            },
            FixerOption {
                name: "classes_opening_brace",
                description: "Position for class/interface/trait braces",
                option_type: OptionType::Enum(vec!["same_line", "next_line_unless_newline_at_signature_end"]),
                default: Some(ConfigValue::String("next_line_unless_newline_at_signature_end".to_string())),
            },
        ]
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Get configuration (PSR-12 defaults)
        let control_style = config.options.get("control_structures_opening_brace")
            .and_then(|v| match v {
                ConfigValue::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("same_line");

        let function_style = config.options.get("functions_opening_brace")
            .and_then(|v| match v {
                ConfigValue::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("next_line_unless_newline_at_signature_end");

        let class_style = config.options.get("classes_opening_brace")
            .and_then(|v| match v {
                ConfigValue::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("next_line_unless_newline_at_signature_end");

        // Control structures (if, for, while, foreach, switch, try, catch) - same line
        if control_style == "same_line" {
            // Fix brace on next line -> same line
            let control_next_line = Regex::new(
                r"(?m)\b(if|elseif|else|for|foreach|while|do|switch|try|catch|finally)\s*(\([^)]*\))?\s*\n\s*\{"
            ).unwrap();

            for cap in control_next_line.captures_iter(source) {
                let full_match = cap.get(0).unwrap();
                let keyword = cap.get(1).unwrap().as_str();
                let condition = cap.get(2).map(|m| m.as_str()).unwrap_or("");

                if is_in_string(&source[..full_match.start()]) {
                    continue;
                }

                let replacement = if condition.is_empty() {
                    format!("{} {{", keyword)
                } else {
                    format!("{} {} {{", keyword, condition)
                };

                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    replacement,
                    format!("Opening brace for {} on same line", keyword),
                    "braces_position",
                ));
            }
        }

        // Functions/methods
        if function_style == "next_line_unless_newline_at_signature_end" {
            // Fix brace on same line -> next line
            let func_same_line = Regex::new(
                r"(?m)(function\s+\w+\s*\([^)]*\)(?:\s*:\s*\??\w+)?)\s*\{"
            ).unwrap();

            for cap in func_same_line.captures_iter(source) {
                let full_match = cap.get(0).unwrap();
                let signature = cap.get(1).unwrap().as_str();

                if is_in_string(&source[..full_match.start()]) {
                    continue;
                }

                // Check if brace is already on next line
                let between_sig_brace = &source[cap.get(1).unwrap().end()..full_match.end()-1];
                if between_sig_brace.contains('\n') {
                    continue;
                }

                // Get indent of current line
                let line_start = source[..full_match.start()].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let indent = &source[line_start..full_match.start()]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect::<String>();

                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    format!("{}{}{}{{", signature, line_ending, indent),
                    "Opening brace for function on next line".to_string(),
                    "braces_position",
                ));
            }
        } else if function_style == "same_line" {
            // Fix brace on next line -> same line
            let func_next_line = Regex::new(
                r"(?m)(function\s+\w+\s*\([^)]*\)(?:\s*:\s*\??\w+)?)\s*\n\s*\{"
            ).unwrap();

            for cap in func_next_line.captures_iter(source) {
                let full_match = cap.get(0).unwrap();
                let signature = cap.get(1).unwrap().as_str();

                if is_in_string(&source[..full_match.start()]) {
                    continue;
                }

                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    format!("{} {{", signature),
                    "Opening brace for function on same line".to_string(),
                    "braces_position",
                ));
            }
        }

        // Classes/interfaces/traits
        if class_style == "next_line_unless_newline_at_signature_end" {
            // Fix brace on same line -> next line
            let class_same_line = Regex::new(
                r"(?m)((?:abstract\s+|final\s+)?(?:class|interface|trait)\s+\w+(?:\s+extends\s+\w+)?(?:\s+implements\s+[\w,\s\\]+)?)\s*\{"
            ).unwrap();

            for cap in class_same_line.captures_iter(source) {
                let full_match = cap.get(0).unwrap();
                let signature = cap.get(1).unwrap().as_str();

                if is_in_string(&source[..full_match.start()]) {
                    continue;
                }

                // Check if brace is already on next line
                let between = &source[cap.get(1).unwrap().end()..full_match.end()-1];
                if between.contains('\n') {
                    continue;
                }

                // Get indent
                let line_start = source[..full_match.start()].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let indent = &source[line_start..full_match.start()]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect::<String>();

                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    format!("{}{}{}{{", signature, line_ending, indent),
                    "Opening brace for class on next line".to_string(),
                    "braces_position",
                ));
            }
        } else if class_style == "same_line" {
            // Fix brace on next line -> same line
            let class_next_line = Regex::new(
                r"(?m)((?:abstract\s+|final\s+)?(?:class|interface|trait)\s+\w+(?:\s+extends\s+\w+)?(?:\s+implements\s+[\w,\s\\]+)?)\s*\n\s*\{"
            ).unwrap();

            for cap in class_next_line.captures_iter(source) {
                let full_match = cap.get(0).unwrap();
                let signature = cap.get(1).unwrap().as_str();

                if is_in_string(&source[..full_match.start()]) {
                    continue;
                }

                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    format!("{} {{", signature),
                    "Opening brace for class on same line".to_string(),
                    "braces_position",
                ));
            }
        }

        // Expand single-line empty bodies to multi-line
        // Pattern: `{ }` or `{}` on same line as control structure should become multi-line
        // Use [^\S\n] to match horizontal whitespace only (not newlines)
        let single_line_empty = Regex::new(
            r"(?m)((?:if|elseif|else|for|foreach|while|do|switch|try|catch|finally)\s*(?:\([^)]*\))?)\s*\{[^\S\n]*\}"
        ).unwrap();

        for cap in single_line_empty.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let prefix = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Get indent of current line
            let line_start = source[..full_match.start()].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let indent: String = source[line_start..full_match.start()]
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{} {{{}{}}}", prefix, line_ending, indent),
                "Expand single-line empty body to multi-line".to_string(),
                "braces_position",
            ));
        }

        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        if c == '\'' && prev_char != '\\' && !in_double_quote {
            in_single_quote = !in_single_quote;
        }
        if c == '"' && prev_char != '\\' && !in_single_quote {
            in_double_quote = !in_double_quote;
        }
        prev_char = c;
    }

    in_single_quote || in_double_quote
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LineEnding;

    fn check(source: &str) -> Vec<Edit> {
        BracesPositionFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        })
    }

    #[test]
    fn test_correct_psr12_class() {
        // Class brace on next line is correct
        let source = "<?php\nclass Foo\n{\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_correct_psr12_if() {
        // Control structure brace on same line is correct
        let source = "<?php\nif (true) {\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_if_brace_next_line() {
        let source = "<?php\nif (true)\n{\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("if (true) {"));
    }

    #[test]
    fn test_class_brace_same_line() {
        let source = "<?php\nclass Foo {\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("class Foo\n{"));
    }

    #[test]
    fn test_function_brace_same_line() {
        let source = "<?php\nfunction foo() {\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("function foo()\n{"));
    }

    #[test]
    fn test_correct_function() {
        let source = "<?php\nfunction foo()\n{\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_while_brace() {
        let source = "<?php\nwhile (true)\n{\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("while (true) {"));
    }

    #[test]
    fn test_foreach_brace() {
        let source = "<?php\nforeach ($a as $b)\n{\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'class Foo {';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_for_brace_next_line() {
        let source = "<?php\nfor($i=0;$i<10;$i++)\n{\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        // Fixer adds a space after keyword: for ($i=0;...) {
        assert!(edits[0].replacement.contains(") {"));
    }

    // Tests for same_line configuration
    fn check_same_line(source: &str) -> Vec<Edit> {
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert("functions_opening_brace".to_string(), ConfigValue::String("same_line".to_string()));
        options.insert("classes_opening_brace".to_string(), ConfigValue::String("same_line".to_string()));
        BracesPositionFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            options,
            ..Default::default()
        })
    }

    #[test]
    fn test_function_next_line_to_same_line() {
        let source = "<?php\nfunction foo()\n{\n}\n";
        let edits = check_same_line(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("function foo() {"));
    }

    #[test]
    fn test_class_next_line_to_same_line() {
        let source = "<?php\nclass Foo\n{\n}\n";
        let edits = check_same_line(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("class Foo {"));
    }

    #[test]
    fn test_function_already_same_line() {
        let source = "<?php\nfunction foo() {\n}\n";
        let edits = check_same_line(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_class_already_same_line() {
        let source = "<?php\nclass Foo {\n}\n";
        let edits = check_same_line(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_class_with_extends_same_line() {
        let source = "<?php\nclass Foo extends Bar\n{\n}\n";
        let edits = check_same_line(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("class Foo extends Bar {"));
    }

    #[test]
    fn test_function_with_return_type_same_line() {
        let source = "<?php\nfunction foo(): string\n{\n}\n";
        let edits = check_same_line(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("function foo(): string {"));
    }

    #[test]
    fn test_single_line_empty_body_expanded() {
        // Single-line empty body should be expanded to multi-line
        let source = "<?php\nif (true) { }";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("if (true) {\n}"));
    }

    #[test]
    fn test_multi_line_empty_body_unchanged() {
        // Multi-line empty body should NOT be changed
        let source = "<?php\nif (true) {\n}\n";
        let edits = check(source);

        assert!(edits.is_empty());
    }
}
