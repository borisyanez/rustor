//! Fix magic method casing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures magic methods use correct casing
pub struct MagicMethodCasingFixer;

// PHP magic methods with their correct casing
const MAGIC_METHODS: &[&str] = &[
    "__construct",
    "__destruct",
    "__call",
    "__callStatic",
    "__get",
    "__set",
    "__isset",
    "__unset",
    "__sleep",
    "__wakeup",
    "__serialize",
    "__unserialize",
    "__toString",
    "__invoke",
    "__set_state",
    "__clone",
    "__debugInfo",
];

impl Fixer for MagicMethodCasingFixer {
    fn name(&self) -> &'static str {
        "magic_method_casing"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "magic_method_casing"
    }

    fn description(&self) -> &'static str {
        "Ensure magic methods use correct casing"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match function declarations that look like magic methods
        let func_re = Regex::new(r"(?i)\bfunction\s+(__[a-z_]+)\s*\(").unwrap();

        for cap in func_re.captures_iter(source) {
            let func_name = cap.get(1).unwrap();
            let func_str = func_name.as_str();
            let func_lower = func_str.to_lowercase();

            // Find the correct casing for this magic method
            let correct_casing = MAGIC_METHODS
                .iter()
                .find(|&&m| m.to_lowercase() == func_lower);

            if let Some(&correct) = correct_casing {
                if func_str != correct {
                    if is_in_string(&source[..func_name.start()]) {
                        continue;
                    }

                    edits.push(edit_with_rule(
                        func_name.start(),
                        func_name.end(),
                        correct.to_string(),
                        format!("Magic method {} should be {}", func_str, correct),
                        "magic_method_casing",
                    ));
                }
            }
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
        MagicMethodCasingFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nclass A { function __construct() {} }\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_uppercase_construct() {
        let source = "<?php\nclass A { function __CONSTRUCT() {} }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__construct");
    }

    #[test]
    fn test_mixed_case_construct() {
        let source = "<?php\nclass A { function __Construct() {} }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__construct");
    }

    #[test]
    fn test_tostring_correct_case() {
        let source = "<?php\nclass A { function __toString() {} }\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_tostring_wrong_case() {
        let source = "<?php\nclass A { function __TOSTRING() {} }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__toString");
    }

    #[test]
    fn test_call_static() {
        let source = "<?php\nclass A { function __CALLSTATIC($name, $args) {} }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__callStatic");
    }

    #[test]
    fn test_destruct() {
        let source = "<?php\nclass A { function __DESTRUCT() {} }\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__destruct");
    }

    #[test]
    fn test_multiple_magic_methods() {
        let source = "<?php\nclass A {\n    function __CONSTRUCT() {}\n    function __TOSTRING() {}\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_non_magic() {
        let source = "<?php\nclass A { function __custom() {} }\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'function __CONSTRUCT()';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
