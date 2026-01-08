//! Include/require fixer

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct IncludeFixer;

impl Fixer for IncludeFixer {
    fn name(&self) -> &'static str { "include" }
    fn php_cs_fixer_name(&self) -> &'static str { "include" }
    fn description(&self) -> &'static str { "Use include/require without parentheses" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match include/require with parentheses
        let keywords = ["include", "include_once", "require", "require_once"];

        for keyword in &keywords {
            let re = Regex::new(&format!(r"\b{}\s*\(\s*([^)]+)\s*\)", keyword)).unwrap();

            for cap in re.captures_iter(source) {
                let full = cap.get(0).unwrap();
                let path = cap.get(1).unwrap().as_str().trim();

                if is_in_string(&source[..full.start()]) { continue; }

                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{} {}", keyword, path),
                    "Remove parentheses from include/require".to_string(),
                    "include",
                ));
            }
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
    fn test_include_parens() {
        let edits = IncludeFixer.check("<?php\ninclude('file.php');", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_include_no_parens() {
        let edits = IncludeFixer.check("<?php\ninclude 'file.php';", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
