//! Replace (unset) cast with null

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoUnsetCastFixer;

impl Fixer for NoUnsetCastFixer {
    fn name(&self) -> &'static str { "no_unset_cast" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_unset_cast" }
    fn description(&self) -> &'static str { "Replace (unset) cast with null" }
    fn priority(&self) -> i32 { 20 }
    fn is_risky(&self) -> bool { true }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        // Match (unset)$var pattern - deprecated in PHP 7.2, removed in 8.0
        let re = Regex::new(r"(?i)\(\s*unset\s*\)\s*\$\w+").unwrap();

        for m in re.find_iter(source) {
            if is_in_string(&source[..m.start()]) { continue; }

            edits.push(edit_with_rule(
                m.start(), m.end(), "null".to_string(),
                "Replace (unset) with null".to_string(),
                "no_unset_cast",
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
    fn test_unset_cast() {
        let edits = NoUnsetCastFixer.check("<?php\n$a = (unset)$b;", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "null");
    }
}
