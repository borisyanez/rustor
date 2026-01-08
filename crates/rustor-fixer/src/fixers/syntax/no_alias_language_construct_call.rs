//! No alias language construct call - die() to exit()

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoAliasLanguageConstructCallFixer;

impl Fixer for NoAliasLanguageConstructCallFixer {
    fn name(&self) -> &'static str { "no_alias_language_construct_call" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_alias_language_construct_call" }
    fn description(&self) -> &'static str { "die() to exit()" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match die()
        let re = Regex::new(r"\bdie\s*\(").unwrap();

        for m in re.find_iter(source) {
            if is_in_string(&source[..m.start()]) { continue; }

            edits.push(edit_with_rule(
                m.start(), m.end(),
                "exit(".to_string(),
                "Use exit() instead of die()".to_string(),
                "no_alias_language_construct_call",
            ));
        }

        // Match die; (without parentheses)
        let re2 = Regex::new(r"\bdie\s*;").unwrap();

        for m in re2.find_iter(source) {
            if is_in_string(&source[..m.start()]) { continue; }

            edits.push(edit_with_rule(
                m.start(), m.end(),
                "exit;".to_string(),
                "Use exit instead of die".to_string(),
                "no_alias_language_construct_call",
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
    fn test_die_to_exit() {
        let edits = NoAliasLanguageConstructCallFixer.check("<?php\ndie('error');", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_exit_unchanged() {
        let edits = NoAliasLanguageConstructCallFixer.check("<?php\nexit('error');", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
