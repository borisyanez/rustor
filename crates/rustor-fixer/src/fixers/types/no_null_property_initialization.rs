//! Remove null initialization for nullable properties

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoNullPropertyInitializationFixer;

impl Fixer for NoNullPropertyInitializationFixer {
    fn name(&self) -> &'static str { "no_null_property_initialization" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_null_property_initialization" }
    fn description(&self) -> &'static str { "Remove null initialization for nullable properties" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: public ?Type $prop = null;
        let re = Regex::new(r"((?:public|protected|private)\s+\?\w+\s+\$\w+)\s*=\s*null\s*;").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let decl = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{};", decl),
                "Remove redundant null initialization".to_string(),
                "no_null_property_initialization",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_remove_null_init() {
        let edits = NoNullPropertyInitializationFixer.check(
            "class A { public ?string $x = null; }",
            &FixerConfig::default()
        );
        assert!(!edits.is_empty());
        assert!(!edits[0].replacement.contains("= null"));
    }

    #[test]
    fn test_non_nullable_unchanged() {
        let edits = NoNullPropertyInitializationFixer.check(
            "class A { public string $x = ''; }",
            &FixerConfig::default()
        );
        assert!(edits.is_empty());
    }
}
