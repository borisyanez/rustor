//! Remove useless concatenation of literal strings

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoUselessConcatOperatorFixer;

impl Fixer for NoUselessConcatOperatorFixer {
    fn name(&self) -> &'static str { "no_useless_concat_operator" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_useless_concat_operator" }
    fn description(&self) -> &'static str { "Remove useless string concatenation" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match 'str1' . 'str2' (single quotes)
        let re1 = Regex::new(r"'([^'\\]*(?:\\.[^'\\]*)*)'\s*\.\s*'([^'\\]*(?:\\.[^'\\]*)*)'").unwrap();
        for cap in re1.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let s1 = cap.get(1).unwrap().as_str();
            let s2 = cap.get(2).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("'{}{}'", s1, s2),
                "Merge concatenated strings".to_string(),
                "no_useless_concat_operator",
            ));
        }

        // Match "str1" . "str2" (double quotes without variables)
        let re2 = Regex::new(r#""([^"$\\]*(?:\\.[^"$\\]*)*)"\s*\.\s*"([^"$\\]*(?:\\.[^"$\\]*)*)""#).unwrap();
        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let s1 = cap.get(1).unwrap().as_str();
            let s2 = cap.get(2).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("\"{}{}\"", s1, s2),
                "Merge concatenated strings".to_string(),
                "no_useless_concat_operator",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_concat_single_quotes() {
        let edits = NoUselessConcatOperatorFixer.check("<?php\n$a = 'foo' . 'bar';", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "'foobar'");
    }

    #[test]
    fn test_concat_double_quotes() {
        let edits = NoUselessConcatOperatorFixer.check("<?php\n$a = \"foo\" . \"bar\";", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "\"foobar\"");
    }
}
