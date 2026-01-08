//! Nullable type declaration fixer

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NullableTypeDeclarationFixer;

impl Fixer for NullableTypeDeclarationFixer {
    fn name(&self) -> &'static str { "nullable_type_declaration" }
    fn php_cs_fixer_name(&self) -> &'static str { "nullable_type_declaration" }
    fn description(&self) -> &'static str { "Normalize nullable type declaration style" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Convert Type|null to ?Type for parameters and return types
        // Match both uppercase class names and lowercase built-in types
        let re = Regex::new(r":\s*([a-zA-Z][a-zA-Z0-9_\\]*)\s*\|\s*null\b").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let type_name = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!(": ?{}", type_name),
                "Use nullable type syntax".to_string(),
                "nullable_type_declaration",
            ));
        }

        // Also convert null|Type to ?Type
        let re2 = Regex::new(r":\s*null\s*\|\s*([a-zA-Z][a-zA-Z0-9_\\]*)").unwrap();

        for cap in re2.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let type_name = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!(": ?{}", type_name),
                "Use nullable type syntax".to_string(),
                "nullable_type_declaration",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_union_to_nullable() {
        let edits = NullableTypeDeclarationFixer.check("function f(): string|null {}", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("?string"));
    }

    #[test]
    fn test_null_first() {
        let edits = NullableTypeDeclarationFixer.check("function f(): null|string {}", &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
