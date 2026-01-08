//! No multiple statements per line

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoMultipleStatementsPerLineFixer;

impl Fixer for NoMultipleStatementsPerLineFixer {
    fn name(&self) -> &'static str { "no_multiple_statements_per_line" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_multiple_statements_per_line" }
    fn description(&self) -> &'static str { "Only one statement per line" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match multiple statements on one line: $a = 1; $b = 2;
        let re = Regex::new(r";(\s*)(\$\w+\s*=)").unwrap();

        for cap in re.captures_iter(source) {
            let space = cap.get(1).unwrap();
            if !space.as_str().contains('\n') {
                let full = cap.get(0).unwrap();
                let stmt = cap.get(2).unwrap().as_str();

                // Get indentation from context
                let before = &source[..full.start()];
                let indent = before.rfind('\n')
                    .map(|pos| {
                        let line_start = pos + 1;
                        let line = &source[line_start..full.start()];
                        line.chars().take_while(|c| c.is_whitespace()).collect::<String>()
                    })
                    .unwrap_or_default();

                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!(";\n{}{}", indent, stmt),
                    "Split into multiple lines".to_string(),
                    "no_multiple_statements_per_line",
                ));
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_split_statements() {
        let edits = NoMultipleStatementsPerLineFixer.check("<?php $a = 1; $b = 2;", &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
