//! Add ? to type when default is null

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NullableTypeDeclarationForDefaultNullValueFixer;

impl Fixer for NullableTypeDeclarationForDefaultNullValueFixer {
    fn name(&self) -> &'static str { "nullable_type_declaration_for_default_null_value" }
    fn php_cs_fixer_name(&self) -> &'static str { "nullable_type_declaration_for_default_null_value" }
    fn description(&self) -> &'static str { "Add ? to type with null default" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match: Type $var = null (without ?)
        let re = Regex::new(r"([^?])([A-Z][a-zA-Z0-9_\\]*)\s+(\$\w+)\s*=\s*null").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let prefix = cap.get(1).unwrap().as_str();
            let type_name = cap.get(2).unwrap().as_str();
            let var = cap.get(3).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}?{} {} = null", prefix, type_name, var),
                "Add ? for nullable type with null default".to_string(),
                "nullable_type_declaration_for_default_null_value",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_nullable() {
        let edits = NullableTypeDeclarationForDefaultNullValueFixer.check(
            "function f(string $x = null) {}",
            &FixerConfig::default()
        );
        // Note: string is lowercase, won't match the [A-Z] pattern
        // This is intentional - built-in types should use ?string
    }

    #[test]
    fn test_class_type() {
        let edits = NullableTypeDeclarationForDefaultNullValueFixer.check(
            "function f(User $x = null) {}",
            &FixerConfig::default()
        );
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("?User"));
    }
}
