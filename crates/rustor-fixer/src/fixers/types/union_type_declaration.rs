//! Normalize union type declarations

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct UnionTypeDeclarationFixer;

impl Fixer for UnionTypeDeclarationFixer {
    fn name(&self) -> &'static str { "union_type_declaration" }
    fn php_cs_fixer_name(&self) -> &'static str { "union_type_declaration" }
    fn description(&self) -> &'static str { "Normalize union type declarations" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Ensure no spaces around | in union types
        let re = Regex::new(r":\s*(\w+)\s+\|\s+(\w+)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let t1 = cap.get(1).unwrap().as_str();
            let t2 = cap.get(2).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!(": {}|{}", t1, t2),
                "Remove spaces around | in union type".to_string(),
                "union_type_declaration",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_remove_spaces() {
        let edits = UnionTypeDeclarationFixer.check("function f(): int | string {}", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("int|string"));
    }

    #[test]
    fn test_already_correct() {
        let edits = UnionTypeDeclarationFixer.check("function f(): int|string {}", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
