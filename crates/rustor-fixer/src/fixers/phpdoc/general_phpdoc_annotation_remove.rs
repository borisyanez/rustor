//! Remove specific PHPDoc annotations

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, ConfigValue, edit_with_rule};

pub struct GeneralPhpdocAnnotationRemoveFixer;

impl Fixer for GeneralPhpdocAnnotationRemoveFixer {
    fn name(&self) -> &'static str { "general_phpdoc_annotation_remove" }
    fn php_cs_fixer_name(&self) -> &'static str { "general_phpdoc_annotation_remove" }
    fn description(&self) -> &'static str { "Remove specific PHPDoc annotations" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Get annotations to remove from config
        let annotations_to_remove = match config.options.get("annotations") {
            Some(ConfigValue::Array(arr)) => arr.clone(),
            _ => vec!["author".to_string(), "package".to_string(), "subpackage".to_string()],
        };

        for annotation in &annotations_to_remove {
            let re = Regex::new(&format!(r"(?m)^\s*\*\s*@{}\b[^\n]*\n", regex::escape(annotation))).unwrap();

            for m in re.find_iter(source) {
                edits.push(edit_with_rule(
                    m.start(), m.end(),
                    String::new(),
                    format!("Remove @{} annotation", annotation),
                    "general_phpdoc_annotation_remove",
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
    fn test_remove_author() {
        let code = "<?php
/**
 * @author John
 */";
        let edits = GeneralPhpdocAnnotationRemoveFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
