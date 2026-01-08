//! Normalize case of PHP constants (true, false, null)

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule, ConfigValue, FixerOption, OptionType};

/// Ensures true, false, null are lowercase (by default) or uppercase
pub struct ConstantCaseFixer;

impl Fixer for ConstantCaseFixer {
    fn name(&self) -> &'static str {
        "constant_case"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "constant_case"
    }

    fn description(&self) -> &'static str {
        "Fix casing of true, false, null constants"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn options(&self) -> Vec<FixerOption> {
        vec![FixerOption {
            name: "case",
            description: "Case to use: 'lower' or 'upper'",
            option_type: OptionType::Enum(vec!["lower", "upper"]),
            default: Some(ConfigValue::String("lower".to_string())),
        }]
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let use_upper = config.options.get("case")
            .and_then(|v| match v {
                ConfigValue::String(s) => Some(s.as_str()),
                _ => None,
            })
            .map(|s| s == "upper")
            .unwrap_or(false);

        let mut edits = Vec::new();

        // Match true, false, null (case insensitive, word boundaries)
        let constants = [
            ("true", "TRUE"),
            ("false", "FALSE"),
            ("null", "NULL"),
        ];

        for (lower, upper) in &constants {
            let pattern = format!(r"(?i)\b({})\b", lower);
            let re = Regex::new(&pattern).unwrap();

            for cap in re.captures_iter(source) {
                let matched = cap.get(1).unwrap();
                let matched_str = matched.as_str();

                let target = if use_upper { *upper } else { *lower };

                // Skip if already correct case
                if matched_str == target {
                    continue;
                }

                // Skip if inside string/comment
                if is_in_string_or_comment(&source[..matched.start()]) {
                    continue;
                }

                edits.push(edit_with_rule(
                    matched.start(),
                    matched.end(),
                    target.to_string(),
                    format!(
                        "Use {} for '{}'",
                        if use_upper { "uppercase" } else { "lowercase" },
                        lower
                    ),
                    "constant_case",
                ));
            }
        }

        edits
    }
}

/// Simple check if position is in string or comment
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
    use std::collections::HashMap;

    fn check(source: &str) -> Vec<Edit> {
        ConstantCaseFixer.check(source, &FixerConfig::default())
    }

    fn check_upper(source: &str) -> Vec<Edit> {
        let mut options = HashMap::new();
        options.insert("case".to_string(), ConfigValue::String("upper".to_string()));
        ConstantCaseFixer.check(source, &FixerConfig {
            options,
            ..Default::default()
        })
    }

    #[test]
    fn test_lowercase_unchanged() {
        let edits = check("<?php\n$a = true;\n$b = false;\n$c = null;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_uppercase_to_lowercase() {
        let source = "<?php\n$a = TRUE;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "true");
    }

    #[test]
    fn test_mixed_case() {
        let source = "<?php\n$a = True;\n$b = FALSE;\n$c = Null;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 3);
    }

    #[test]
    fn test_upper_option() {
        let source = "<?php\n$a = true;\n";
        let edits = check_upper(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "TRUE");
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'TRUE';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_in_expression() {
        let source = "<?php\nif ($a === NULL) { return FALSE; }\n";
        let edits = check(source);

        assert!(edits.iter().any(|e| e.replacement == "null"));
        assert!(edits.iter().any(|e| e.replacement == "false"));
    }
}
