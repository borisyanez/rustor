//! Remove useless nullsafe operator when value cannot be null

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoUselessNullsafeOperatorFixer;

impl Fixer for NoUselessNullsafeOperatorFixer {
    fn name(&self) -> &'static str { "no_useless_nullsafe_operator" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_useless_nullsafe_operator" }
    fn description(&self) -> &'static str { "Remove nullsafe when value cannot be null" }
    fn priority(&self) -> i32 { 20 }
    fn is_risky(&self) -> bool { true }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match new Foo()?->method() - new cannot return null
        let re = Regex::new(r"(new\s+[A-Z][a-zA-Z0-9_\\]*\s*\([^)]*\))\s*\?->").unwrap();
        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let new_expr = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}->", new_expr),
                "Remove useless nullsafe after new".to_string(),
                "no_useless_nullsafe_operator",
            ));
        }

        // Match clone $var?-> - clone cannot return null
        let re2 = Regex::new(r"(clone\s+\$\w+)\s*\?->").unwrap();
        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let clone_expr = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}->", clone_expr),
                "Remove useless nullsafe after clone".to_string(),
                "no_useless_nullsafe_operator",
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
    fn test_new_nullsafe() {
        let edits = NoUselessNullsafeOperatorFixer.check("<?php\n$a = new Foo()?->bar();", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("new Foo()->"));
    }

    #[test]
    fn test_clone_nullsafe() {
        let edits = NoUselessNullsafeOperatorFixer.check("<?php\n$a = clone $obj?->foo;", &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
