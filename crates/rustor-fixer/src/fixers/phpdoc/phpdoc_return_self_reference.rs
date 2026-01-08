//! Use $this or static instead of class name in @return

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocReturnSelfReferenceFixer;

impl Fixer for PhpdocReturnSelfReferenceFixer {
    fn name(&self) -> &'static str { "phpdoc_return_self_reference" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_return_self_reference" }
    fn description(&self) -> &'static str { "Use $this or static in @return" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Find class name
        let class_re = Regex::new(r"class\s+(\w+)").unwrap();

        for class_cap in class_re.captures_iter(source) {
            let class_name = class_cap.get(1).unwrap().as_str();
            let class_start = class_cap.get(0).unwrap().start();

            // Find @return ClassName in this class
            let return_re = Regex::new(&format!(r"(@return\s+){}\b", regex::escape(class_name))).unwrap();

            for ret_cap in return_re.captures_iter(&source[class_start..]) {
                let full = ret_cap.get(0).unwrap();
                let prefix = ret_cap.get(1).unwrap().as_str();

                edits.push(edit_with_rule(
                    class_start + full.start(), class_start + full.end(),
                    format!("{}$this", prefix),
                    "Use $this instead of class name in @return".to_string(),
                    "phpdoc_return_self_reference",
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
    fn test_self_reference() {
        let code = "<?php
class Foo {
    /**
     * @return Foo
     */
    public function bar() {}
}";
        let edits = PhpdocReturnSelfReferenceFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("$this"));
    }
}
