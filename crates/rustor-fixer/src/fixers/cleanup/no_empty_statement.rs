//! Remove empty statements (standalone semicolons)

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoEmptyStatementFixer;

impl Fixer for NoEmptyStatementFixer {
    fn name(&self) -> &'static str { "no_empty_statement" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_empty_statement" }
    fn description(&self) -> &'static str { "Remove empty statements" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        // Match double semicolons or semicolon after opening brace
        let patterns = [
            (r";\s*;", ";"),
            (r"\{\s*;", "{"),
        ];

        for (pattern, replacement) in patterns {
            let re = Regex::new(pattern).unwrap();
            for m in re.find_iter(source) {
                if is_in_string(&source[..m.start()]) { continue; }
                edits.push(edit_with_rule(
                    m.start(), m.end(), replacement.to_string(),
                    "Remove empty statement".to_string(), "no_empty_statement",
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
    fn test_double_semicolon() {
        let edits = NoEmptyStatementFixer.check("<?php\n$a = 1;;", &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
