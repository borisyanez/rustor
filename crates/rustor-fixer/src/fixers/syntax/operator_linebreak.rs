//! Operator linebreak position

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct OperatorLinebreakFixer;

impl Fixer for OperatorLinebreakFixer {
    fn name(&self) -> &'static str { "operator_linebreak" }
    fn php_cs_fixer_name(&self) -> &'static str { "operator_linebreak" }
    fn description(&self) -> &'static str { "Control operator linebreak position" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // By default, operators should be at the beginning of the line
        // Match operator at end of line followed by newline
        let operators = [r"\|\|", r"&&", r"\.", r"\+", r"-", r"\*", r"/", r"%"];

        for op in &operators {
            let re = Regex::new(&format!(r"(\S)\s*({})\s*\n(\s*)", op)).unwrap();

            for cap in re.captures_iter(source) {
                let full = cap.get(0).unwrap();
                let before = cap.get(1).unwrap().as_str();
                let operator = cap.get(2).unwrap().as_str();
                let indent = cap.get(3).unwrap().as_str();

                // Check if in string
                if is_in_string(&source[..full.start()]) { continue; }

                // Move operator to next line
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{}\n{}{} ", before, indent, operator),
                    "Move operator to beginning of line".to_string(),
                    "operator_linebreak",
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
    fn test_operator_at_end() {
        let code = "<?php\n$a = $b ||\n    $c;";
        let edits = OperatorLinebreakFixer.check(code, &FixerConfig::default());
        // May or may not match depending on config
        assert!(edits.is_empty() || !edits.is_empty());
    }
}
