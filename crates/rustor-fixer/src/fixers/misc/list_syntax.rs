//! Use short list syntax []

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct ListSyntaxFixer;

impl Fixer for ListSyntaxFixer {
    fn name(&self) -> &'static str { "list_syntax" }
    fn php_cs_fixer_name(&self) -> &'static str { "list_syntax" }
    fn description(&self) -> &'static str { "Use short list syntax" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: list($a, $b) = ...
        let re = Regex::new(r"\blist\s*\(([^)]+)\)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let vars = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("[{}]", vars),
                "Use short list syntax".to_string(),
                "list_syntax",
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
    fn test_list_to_short() {
        let edits = ListSyntaxFixer.check("<?php\nlist($a, $b) = $arr;", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("[$a, $b]"));
    }
}
