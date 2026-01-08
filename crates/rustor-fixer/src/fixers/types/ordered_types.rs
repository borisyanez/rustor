//! Order union types alphabetically

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct OrderedTypesFixer;

impl Fixer for OrderedTypesFixer {
    fn name(&self) -> &'static str { "ordered_types" }
    fn php_cs_fixer_name(&self) -> &'static str { "ordered_types" }
    fn description(&self) -> &'static str { "Order union types alphabetically" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match union types: int|string|null
        let re = Regex::new(r":\s*([a-zA-Z_\\]+(?:\s*\|\s*[a-zA-Z_\\]+)+)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let types_str = cap.get(1).unwrap().as_str();

            let mut types: Vec<&str> = types_str.split('|')
                .map(|t| t.trim())
                .collect();

            let original = types.clone();

            // Sort with null last
            types.sort_by(|a, b| {
                if *a == "null" { std::cmp::Ordering::Greater }
                else if *b == "null" { std::cmp::Ordering::Less }
                else { a.to_lowercase().cmp(&b.to_lowercase()) }
            });

            if types != original {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!(": {}", types.join("|")),
                    "Order union types".to_string(),
                    "ordered_types",
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
    fn test_order_types() {
        let edits = OrderedTypesFixer.check("function f(): null|string|int {}", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("int|string|null"));
    }

    #[test]
    fn test_already_ordered() {
        let edits = OrderedTypesFixer.check("function f(): int|string|null {}", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
