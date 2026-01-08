//! Remove useless return at end of function

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoUselessReturnFixer;

impl Fixer for NoUselessReturnFixer {
    fn name(&self) -> &'static str { "no_useless_return" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_useless_return" }
    fn description(&self) -> &'static str { "Remove useless return statement at end of function" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        // Match standalone return; at end of function body (before closing brace)
        let re = Regex::new(r"(?m)\n\s*return\s*;\s*\n(\s*)\}").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let indent = cap.get(1).unwrap().as_str();
            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("\n{}}}", indent),
                "Remove useless return".to_string(),
                "no_useless_return",
            ));
        }
        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_useless_return() {
        let code = "<?php
function foo() {
    echo 'hi';
    return;
}";
        let edits = NoUselessReturnFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_return_with_value_unchanged() {
        let edits = NoUselessReturnFixer.check("<?php\nfunction f() { return 1; }", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
