//! Cast spaces fixer - control spacing after casts

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct CastSpacesFixer;

impl Fixer for CastSpacesFixer {
    fn name(&self) -> &'static str { "cast_spaces" }
    fn php_cs_fixer_name(&self) -> &'static str { "cast_spaces" }
    fn description(&self) -> &'static str { "Fix spacing after type casts" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match casts with no space after: (int)$x -> (int) $x
        let casts = ["int", "integer", "bool", "boolean", "float", "double", "real", "string", "array", "object", "unset", "binary"];

        for cast in &casts {
            let re = Regex::new(&format!(r"(?i)\(\s*{}\s*\)(\S)", cast)).unwrap();
            for cap in re.captures_iter(source) {
                let full = cap.get(0).unwrap();
                let after = cap.get(1).unwrap().as_str();

                // Don't add space if already has one
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("({}) {}", cast, after),
                    "Add space after cast".to_string(),
                    "cast_spaces",
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
    fn test_add_space_after_cast() {
        let edits = CastSpacesFixer.check("<?php\n$a = (int)$b;", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("(int) $"));
    }

    #[test]
    fn test_space_already_present() {
        let edits = CastSpacesFixer.check("<?php\n$a = (int) $b;", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
