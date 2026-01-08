//! Fix magic constant casing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures magic constants use correct (uppercase) casing
pub struct MagicConstantCasingFixer;

// PHP magic constants (should all be uppercase)
const MAGIC_CONSTANTS: &[&str] = &[
    "__CLASS__",
    "__DIR__",
    "__FILE__",
    "__FUNCTION__",
    "__LINE__",
    "__METHOD__",
    "__NAMESPACE__",
    "__TRAIT__",
];

impl Fixer for MagicConstantCasingFixer {
    fn name(&self) -> &'static str {
        "magic_constant_casing"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "magic_constant_casing"
    }

    fn description(&self) -> &'static str {
        "Ensure magic constants are uppercase"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match magic constants with wrong casing
        let const_re = Regex::new(r"(?i)\b(__(?:CLASS|DIR|FILE|FUNCTION|LINE|METHOD|NAMESPACE|TRAIT)__)\b").unwrap();

        for cap in const_re.captures_iter(source) {
            let const_match = cap.get(1).unwrap();
            let const_str = const_match.as_str();
            let const_upper = const_str.to_uppercase();

            // Find the correct constant
            let correct = MAGIC_CONSTANTS
                .iter()
                .find(|&&c| c == const_upper);

            if let Some(&correct_const) = correct {
                if const_str != correct_const {
                    if is_in_string(&source[..const_match.start()]) {
                        continue;
                    }

                    edits.push(edit_with_rule(
                        const_match.start(),
                        const_match.end(),
                        correct_const.to_string(),
                        format!("Magic constant {} should be uppercase", const_str),
                        "magic_constant_casing",
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
        MagicConstantCasingFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\necho __CLASS__;\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_lowercase_class() {
        let source = "<?php\necho __class__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__CLASS__");
    }

    #[test]
    fn test_mixed_case() {
        let source = "<?php\necho __Class__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__CLASS__");
    }

    #[test]
    fn test_file_constant() {
        let source = "<?php\necho __file__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__FILE__");
    }

    #[test]
    fn test_dir_constant() {
        let source = "<?php\necho __dir__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__DIR__");
    }

    #[test]
    fn test_line_constant() {
        let source = "<?php\necho __line__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__LINE__");
    }

    #[test]
    fn test_function_constant() {
        let source = "<?php\necho __function__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__FUNCTION__");
    }

    #[test]
    fn test_method_constant() {
        let source = "<?php\necho __method__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__METHOD__");
    }

    #[test]
    fn test_namespace_constant() {
        let source = "<?php\necho __namespace__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__NAMESPACE__");
    }

    #[test]
    fn test_trait_constant() {
        let source = "<?php\necho __trait__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "__TRAIT__");
    }

    #[test]
    fn test_multiple_constants() {
        let source = "<?php\necho __class__ . __method__;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = '__class__';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
