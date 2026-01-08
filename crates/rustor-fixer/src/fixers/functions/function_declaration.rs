//! Fix function declaration spacing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures proper spacing in function declarations
pub struct FunctionDeclarationFixer;

impl Fixer for FunctionDeclarationFixer {
    fn name(&self) -> &'static str {
        "function_declaration"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "function_declaration"
    }

    fn description(&self) -> &'static str {
        "Fix function declaration spacing"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Fix space between function name and opening parenthesis
        // function foo () -> function foo()
        let space_before_paren = Regex::new(r"\bfunction\s+(\w+)\s+\(").unwrap();

        for cap in space_before_paren.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let func_name = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("function {}(", func_name),
                "Remove space before function parenthesis".to_string(),
                "function_declaration",
            ));
        }

        // Fix multiple spaces after function keyword
        // function  foo() -> function foo()
        let multi_space = Regex::new(r"\bfunction[ \t]{2,}(\w+)\s*\(").unwrap();

        for cap in multi_space.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let func_name = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Only add edit if not already handled
            let already_edited = edits.iter().any(|e| {
                e.start_offset() <= full_match.start() && e.end_offset() >= full_match.end()
            });

            if !already_edited {
                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    format!("function {}(", func_name),
                    "Single space after function keyword".to_string(),
                    "function_declaration",
                ));
            }
        }

        // Fix closure spacing: function() -> function ()  (configurable, PSR-12 wants no space)
        // But actually PSR-12 says NO space for closures too
        // function () use -> function() use
        let closure_space = Regex::new(r"\bfunction\s+\(\s*\)\s*(use)").unwrap();

        for cap in closure_space.captures_iter(source) {
            let full_match = cap.get(0).unwrap();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                "function() use".to_string(),
                "No space before closure parenthesis".to_string(),
                "function_declaration",
            ));
        }

        // Fix space in empty parameter list
        // function foo( ) -> function foo()
        let empty_params_space = Regex::new(r"\bfunction\s+(\w+)\(\s+\)").unwrap();

        for cap in empty_params_space.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let func_name = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("function {}()", func_name),
                "Remove space in empty parameter list".to_string(),
                "function_declaration",
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

    fn check(source: &str) -> Vec<Edit> {
        FunctionDeclarationFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nfunction foo() {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_space_before_paren() {
        let source = "<?php\nfunction foo () {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("function foo("));
    }

    #[test]
    fn test_multiple_spaces_after_function() {
        let source = "<?php\nfunction  foo() {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("function foo("));
    }

    #[test]
    fn test_empty_params_with_space() {
        let source = "<?php\nfunction foo( ) {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("function foo()"));
    }

    #[test]
    fn test_method_declaration() {
        let source = "<?php\nclass A { public function bar () {} }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'function foo () {}';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
