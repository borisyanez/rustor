//! PHPUnit method casing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpUnitMethodCasingFixer;

impl Fixer for PhpUnitMethodCasingFixer {
    fn name(&self) -> &'static str { "php_unit_method_casing" }
    fn php_cs_fixer_name(&self) -> &'static str { "php_unit_method_casing" }
    fn description(&self) -> &'static str { "Test method casing style" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Default style is camelCase for test methods
        // Match test methods with snake_case: test_something
        let re = Regex::new(r"(?m)function\s+(test_[a-z0-9_]+)\s*\(").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let method = cap.get(1).unwrap().as_str();

            // Convert test_some_thing to testSomeThing
            let camel = snake_to_camel(method);

            if method != camel {
                let new_decl = format!("function {} (", camel);
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    new_decl,
                    "Use camelCase for test methods".to_string(),
                    "php_unit_method_casing",
                ));
            }
        }

        edits
    }
}

fn snake_to_camel(s: &str) -> String {
    let parts: Vec<&str> = s.split('_').collect();
    let mut result = String::new();

    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            result.push_str(part);
        } else {
            // Capitalize first letter
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_ascii_uppercase());
                result.push_str(chars.as_str());
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_snake_case() {
        let code = "<?php\nfunction test_something_works() {}";
        let edits = PhpUnitMethodCasingFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_camel_case() {
        let code = "<?php\nfunction testSomethingWorks() {}";
        let edits = PhpUnitMethodCasingFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }

    #[test]
    fn test_conversion() {
        assert_eq!(snake_to_camel("test_some_thing"), "testSomeThing");
        assert_eq!(snake_to_camel("test_it"), "testIt");
    }
}
