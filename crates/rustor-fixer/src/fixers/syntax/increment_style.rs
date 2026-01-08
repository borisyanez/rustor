//! Increment style fixer - ++$i vs $i++

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct IncrementStyleFixer;

impl Fixer for IncrementStyleFixer {
    fn name(&self) -> &'static str { "increment_style" }
    fn php_cs_fixer_name(&self) -> &'static str { "increment_style" }
    fn description(&self) -> &'static str { "Use pre-increment/decrement when possible" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match standalone $i++ or $i-- (not in expression)
        // Pattern: statement starting with $var++ or $var--;
        let re = Regex::new(r"(?m)^\s*(\$\w+)(\+\+|--)\s*;").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let var = cap.get(1).unwrap().as_str();
            let op = cap.get(2).unwrap().as_str();

            let indent = &full.as_str()[..full.as_str().find('$').unwrap_or(0)];

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}{}{};", indent, op, var),
                "Use pre-increment style".to_string(),
                "increment_style",
            ));
        }
        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_post_to_pre() {
        let edits = IncrementStyleFixer.check("<?php\n$i++;", &FixerConfig::default());
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("++$i"));
    }

    #[test]
    fn test_pre_unchanged() {
        let edits = IncrementStyleFixer.check("<?php\n++$i;", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
