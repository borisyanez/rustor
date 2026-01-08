//! PHPUnit FQCN annotation

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpUnitFqcnAnnotationFixer;

impl Fixer for PhpUnitFqcnAnnotationFixer {
    fn name(&self) -> &'static str { "php_unit_fqcn_annotation" }
    fn php_cs_fixer_name(&self) -> &'static str { "php_unit_fqcn_annotation" }
    fn description(&self) -> &'static str { "Use FQCN in PHPUnit annotations" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match @covers, @coversDefaultClass, @uses without leading backslash
        let annotations = ["covers", "coversDefaultClass", "uses"];

        for annotation in annotations {
            // Match @annotation ClassName (without leading \) followed by end of line or space
            let re = Regex::new(&format!(r"(@{})\s+([A-Z][\\a-zA-Z0-9_]+)(\s|$|\n|\*)", annotation)).unwrap();

            for cap in re.captures_iter(source) {
                let tag = cap.get(1).unwrap();
                let class = cap.get(2).unwrap();
                let class_str = class.as_str();

                // Skip if already has leading backslash
                if !class_str.starts_with('\\') {
                    edits.push(edit_with_rule(
                        tag.start(), class.end(),
                        format!("{} \\{}", tag.as_str(), class_str),
                        "Use FQCN (leading backslash) in PHPUnit annotation".to_string(),
                        "php_unit_fqcn_annotation",
                    ));
                }
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_no_fqcn() {
        let code = "<?php\n/**\n * @covers MyClass\n */";
        let edits = PhpUnitFqcnAnnotationFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_has_fqcn() {
        let code = "<?php\n/**\n * @covers \\MyClass\n */";
        let edits = PhpUnitFqcnAnnotationFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }

    #[test]
    fn test_namespace() {
        let code = "<?php\n/**\n * @covers App\\Service\\MyService\n */";
        let edits = PhpUnitFqcnAnnotationFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
