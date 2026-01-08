//! Replace short bool cast !! with (bool)

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoShortBoolCastFixer;

impl Fixer for NoShortBoolCastFixer {
    fn name(&self) -> &'static str { "no_short_bool_cast" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_short_bool_cast" }
    fn description(&self) -> &'static str { "Replace !! with (bool) cast" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        // Match !!$var or !!(...) pattern
        let re = Regex::new(r"!!\s*(\$\w+|\([^)]+\))").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let expr = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("(bool) {}", expr),
                "Use (bool) instead of !!".to_string(),
                "no_short_bool_cast",
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
    fn test_double_not_to_bool() {
        let edits = NoShortBoolCastFixer.check("<?php\n$b = !!$a;", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("(bool)"));
    }
}
