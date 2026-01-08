//! Simple to complex string variable

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct SimpleToComplexStringVariableFixer;

impl Fixer for SimpleToComplexStringVariableFixer {
    fn name(&self) -> &'static str { "simple_to_complex_string_variable" }
    fn php_cs_fixer_name(&self) -> &'static str { "simple_to_complex_string_variable" }
    fn description(&self) -> &'static str { "$var to {$var} in strings" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Find double-quoted strings
        let string_re = Regex::new(r#""([^"\\]|\\.)*""#).unwrap();

        for string_match in string_re.find_iter(source) {
            let string = string_match.as_str();
            let start = string_match.start();

            // Find simple variables: $var
            let var_re = Regex::new(r"(\$\w+)").unwrap();

            for cap in var_re.captures_iter(string) {
                let full = cap.get(0).unwrap();
                let var = full.as_str();
                let pos_in_string = full.start();

                // Skip if preceded by { (already complex)
                if pos_in_string > 0 {
                    let prev_char = string.chars().nth(pos_in_string - 1);
                    if prev_char == Some('{') {
                        continue;
                    }
                }

                // Skip if followed by [ or -> (array access or property)
                if pos_in_string + var.len() < string.len() {
                    let rest = &string[pos_in_string + var.len()..];
                    if rest.starts_with('[') || rest.starts_with("->") || rest.starts_with('}') {
                        continue;
                    }
                }

                edits.push(edit_with_rule(
                    start + full.start(), start + full.end(),
                    format!("{{{}}}", var),
                    "Use curly brace syntax for variables in strings".to_string(),
                    "simple_to_complex_string_variable",
                ));
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_simple_var() {
        let code = r#"<?php $x = "hello $name";"#;
        let edits = SimpleToComplexStringVariableFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_complex_var() {
        let code = r#"<?php $x = "hello {$name}";"#;
        let edits = SimpleToComplexStringVariableFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
