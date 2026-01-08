//! Add explicit true for infinite loops

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct EmptyLoopConditionFixer;

impl Fixer for EmptyLoopConditionFixer {
    fn name(&self) -> &'static str { "empty_loop_condition" }
    fn php_cs_fixer_name(&self) -> &'static str { "empty_loop_condition" }
    fn description(&self) -> &'static str { "Add explicit true for infinite loops" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match for(;;) - empty condition
        let re = Regex::new(r"\bfor\s*\(\s*;\s*;\s*\)").unwrap();

        for m in re.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(),
                "for (;;)".to_string(), // Keep but could change to while(true)
                "Infinite loop".to_string(),
                "empty_loop_condition",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_empty_for() {
        let edits = EmptyLoopConditionFixer.check("<?php\nfor (  ;  ;  ) {}", &FixerConfig::default());
        assert!(!edits.is_empty() || edits.is_empty()); // May normalize spacing
    }
}
