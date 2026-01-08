//! Class attributes separation fixer

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct ClassAttributesSeparationFixer;

impl Fixer for ClassAttributesSeparationFixer {
    fn name(&self) -> &'static str { "class_attributes_separation" }
    fn php_cs_fixer_name(&self) -> &'static str { "class_attributes_separation" }
    fn description(&self) -> &'static str { "Add blank lines between class members" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match method followed by method without blank line
        let re = Regex::new(r"(\}\n)([ \t]*)(public|protected|private|function)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let close = cap.get(1).unwrap().as_str();
            let indent = cap.get(2).unwrap().as_str();
            let keyword = cap.get(3).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("{}\n{}{}", close, indent, keyword),
                "Add blank line between class members".to_string(),
                "class_attributes_separation",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_method_separation() {
        let code = "<?php
class Foo {
    public function a() {}
    public function b() {}
}";
        let edits = ClassAttributesSeparationFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
