//! Normalize index brace fixer - converts {0} to [0]

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NormalizeIndexBraceFixer;

impl Fixer for NormalizeIndexBraceFixer {
    fn name(&self) -> &'static str { "normalize_index_brace" }
    fn php_cs_fixer_name(&self) -> &'static str { "normalize_index_brace" }
    fn description(&self) -> &'static str { "Use [] instead of {} for array access" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        // Match $var{index} pattern
        let re = Regex::new(r"(\$\w+)\{([^}]+)\}").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let var = cap.get(1).unwrap().as_str();
            let idx = cap.get(2).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}[{}]", var, idx),
                "Use [] instead of {} for array access".to_string(),
                "normalize_index_brace",
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
    fn test_brace_to_bracket() {
        let edits = NormalizeIndexBraceFixer.check("<?php\n$a{0};", &FixerConfig::default());
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$a[0]"));
    }
}
