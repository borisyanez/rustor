//! Use explicit variable syntax in strings

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct ExplicitStringVariableFixer;

impl Fixer for ExplicitStringVariableFixer {
    fn name(&self) -> &'static str { "explicit_string_variable" }
    fn php_cs_fixer_name(&self) -> &'static str { "explicit_string_variable" }
    fn description(&self) -> &'static str { "Use explicit variable syntax in strings" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: "$var" -> "{$var}"
        // Look for variables in double-quoted strings that aren't already wrapped
        let re = Regex::new(r#""([^"]*)\$(\w+)([^"]*?)""#).unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let before = cap.get(1).unwrap().as_str();
            let var = cap.get(2).unwrap().as_str();
            let after = cap.get(3).unwrap().as_str();

            // Skip if already wrapped in {}
            if before.ends_with('{') && after.starts_with('}') { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("\"{}{{${}}}{}\"", before, var, after),
                "Use explicit variable syntax".to_string(),
                "explicit_string_variable",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_explicit_var() {
        let edits = ExplicitStringVariableFixer.check(r#"<?php $a = "hello $name";"#, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("{$name}"));
    }

    #[test]
    fn test_already_explicit() {
        let edits = ExplicitStringVariableFixer.check(r#"<?php $a = "hello {$name}";"#, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
