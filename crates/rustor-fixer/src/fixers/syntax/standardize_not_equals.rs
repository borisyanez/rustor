//! Standardize not equals - <> to !=

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct StandardizeNotEqualsFixer;

impl Fixer for StandardizeNotEqualsFixer {
    fn name(&self) -> &'static str { "standardize_not_equals" }
    fn php_cs_fixer_name(&self) -> &'static str { "standardize_not_equals" }
    fn description(&self) -> &'static str { "Use != instead of <>" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let re = Regex::new(r"<>").unwrap();

        for m in re.find_iter(source) {
            if is_in_string(&source[..m.start()]) { continue; }

            edits.push(edit_with_rule(
                m.start(), m.end(), "!=".to_string(),
                "Use != instead of <>".to_string(),
                "standardize_not_equals",
            ));
        }
        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let (mut s, mut d, mut p) = (false, false, '\0');
    for c in before.chars() {
        if c == '\'' && p != '\\' && !d { s = !s; }
        if c == '"' && p != '\\' && !s { d = !d; }
        p = c;
    }
    s || d
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_diamond_to_not_equals() {
        let edits = StandardizeNotEqualsFixer.check("<?php\nif ($a <> $b) {}", &FixerConfig::default());
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "!=");
    }
}
