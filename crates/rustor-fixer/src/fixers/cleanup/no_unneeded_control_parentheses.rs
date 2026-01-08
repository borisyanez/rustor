//! Remove unneeded parentheses in control structures

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoUnneededControlParenthesesFixer;

impl Fixer for NoUnneededControlParenthesesFixer {
    fn name(&self) -> &'static str { "no_unneeded_control_parentheses" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_unneeded_control_parentheses" }
    fn description(&self) -> &'static str { "Remove unneeded parentheses in control structures" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: return ($var); or return ($expr);
        let re = Regex::new(r"\b(return|yield|yield from|throw|echo|print|include|require|include_once|require_once)\s+\(([^()]+)\)\s*;").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let keyword = cap.get(1).unwrap().as_str();
            let inner = cap.get(2).unwrap().as_str().trim();

            // Skip if inner contains operators that might change precedence
            if inner.contains("?") && inner.contains(":") { continue; } // ternary

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{} {};", keyword, inner),
                "Remove unneeded parentheses".to_string(),
                "no_unneeded_control_parentheses",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_return_parens() {
        let edits = NoUnneededControlParenthesesFixer.check("<?php\nreturn ($x);", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "return $x;");
    }

    #[test]
    fn test_echo_parens() {
        let edits = NoUnneededControlParenthesesFixer.check("<?php\necho ($msg);", &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
