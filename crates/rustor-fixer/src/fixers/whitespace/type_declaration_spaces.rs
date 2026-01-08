//! Spacing around type declarations

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct TypeDeclarationSpacesFixer;

impl Fixer for TypeDeclarationSpacesFixer {
    fn name(&self) -> &'static str { "type_declaration_spaces" }
    fn php_cs_fixer_name(&self) -> &'static str { "type_declaration_spaces" }
    fn description(&self) -> &'static str { "Spacing around type declarations" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match type hint with multiple spaces before variable
        let re = Regex::new(r"(\??\w+)\s{2,}(\$\w+)").unwrap();
        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let type_hint = cap.get(1).unwrap().as_str();
            let var = cap.get(2).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{} {}", type_hint, var),
                "Single space between type and variable".to_string(),
                "type_declaration_spaces",
            ));
        }

        // Match return type with no space after colon
        let re2 = Regex::new(r"\):([a-zA-Z?])").unwrap();
        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let type_start = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("): {}", type_start),
                "Add space after colon in return type".to_string(),
                "type_declaration_spaces",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_multiple_spaces() {
        let edits = TypeDeclarationSpacesFixer.check("function f(int  $x) {}", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_return_type_spacing() {
        let edits = TypeDeclarationSpacesFixer.check("function f():int {}", &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
