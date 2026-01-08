//! Add braces for empty loop body

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct EmptyLoopBodyFixer;

impl Fixer for EmptyLoopBodyFixer {
    fn name(&self) -> &'static str { "empty_loop_body" }
    fn php_cs_fixer_name(&self) -> &'static str { "empty_loop_body" }
    fn description(&self) -> &'static str { "Add braces for empty loop body" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match while/for with empty semicolon body
        let re = Regex::new(r"\b(while|for)\s*\([^)]+\)\s*;").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let text = full.as_str();

            // Replace semicolon with empty braces
            let new_text = text.trim_end_matches(';').to_string() + " {\n    // Empty loop\n}";

            edits.push(edit_with_rule(
                full.start(), full.end(),
                new_text,
                "Add braces for empty loop body".to_string(),
                "empty_loop_body",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_empty_while() {
        let edits = EmptyLoopBodyFixer.check("<?php\nwhile (true);", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_while_with_body() {
        let edits = EmptyLoopBodyFixer.check("<?php\nwhile (true) { break; }", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
