//! Yoda style fixer
//!
//! Converts comparisons to Yoda style (constant on left).

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Converts comparisons to Yoda style
pub struct YodaStyleFixer;

impl Fixer for YodaStyleFixer {
    fn name(&self) -> &'static str {
        "yoda_style"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "yoda_style"
    }

    fn description(&self) -> &'static str {
        "Use Yoda style comparisons (constant on left)"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match $var === null, $var === true, $var === false
        // Convert to null === $var, true === $var, false === $var
        let patterns = [
            (r"(\$\w+)\s*(===|==|!==|!=)\s*(null)\b", true),
            (r"(\$\w+)\s*(===|==|!==|!=)\s*(true)\b", true),
            (r"(\$\w+)\s*(===|==|!==|!=)\s*(false)\b", true),
            // Also match numeric literals
            (r"(\$\w+)\s*(===|==|!==|!=)\s*(\d+)\b", true),
            // Match string literals
            (r#"(\$\w+)\s*(===|==|!==|!=)\s*('[^']*')"#, true),
            (r#"(\$\w+)\s*(===|==|!==|!=)\s*("[^"]*")"#, true),
        ];

        for (pattern, _) in patterns {
            let re = Regex::new(pattern).unwrap();
            for cap in re.captures_iter(source) {
                let full_match = cap.get(0).unwrap();
                let var = cap.get(1).unwrap().as_str();
                let op = cap.get(2).unwrap().as_str();
                let constant = cap.get(3).unwrap().as_str();

                if is_in_string(&source[..full_match.start()]) {
                    continue;
                }

                if is_in_comment(&source[..full_match.start()]) {
                    continue;
                }

                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    format!("{} {} {}", constant, op, var),
                    "Use Yoda style comparison".to_string(),
                    "yoda_style",
                ));
            }
        }

        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';
    for c in before.chars() {
        if c == '\'' && prev != '\\' && !in_double { in_single = !in_single; }
        if c == '"' && prev != '\\' && !in_single { in_double = !in_double; }
        prev = c;
    }
    in_single || in_double
}

fn is_in_comment(before: &str) -> bool {
    if let Some(pos) = before.rfind('\n') {
        let line = &before[pos..];
        if line.contains("//") || line.contains('#') { return true; }
    }
    before.matches("/*").count() > before.matches("*/").count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        YodaStyleFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_yoda_style_unchanged() {
        let source = "<?php\nif (null === $a) {}";
        assert!(check(source).is_empty());
    }

    #[test]
    fn test_null_comparison() {
        let source = "<?php\nif ($a === null) {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("null === $a"));
    }

    #[test]
    fn test_true_comparison() {
        let source = "<?php\nif ($flag == true) {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("true == $flag"));
    }

    #[test]
    fn test_false_comparison() {
        let source = "<?php\nif ($flag !== false) {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("false !== $flag"));
    }

    #[test]
    fn test_numeric_comparison() {
        let source = "<?php\nif ($count === 0) {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("0 === $count"));
    }

    #[test]
    fn test_string_comparison() {
        let source = "<?php\nif ($name === 'test') {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = '$x === null';";
        assert!(check(source).is_empty());
    }
}
