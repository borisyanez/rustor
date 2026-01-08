//! Single line throw

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct SingleLineThrowFixer;

impl Fixer for SingleLineThrowFixer {
    fn name(&self) -> &'static str { "single_line_throw" }
    fn php_cs_fixer_name(&self) -> &'static str { "single_line_throw" }
    fn description(&self) -> &'static str { "Throw on single line" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match throw with unnecessary newlines
        let re = Regex::new(r"throw\s+new\s+\n\s*(\w+)").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let class = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("throw new {}", class),
                "Put throw on single line".to_string(),
                "single_line_throw",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_multiline_throw() {
        let code = "<?php\nthrow new \n    Exception();";
        let edits = SingleLineThrowFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_single_line_throw() {
        let code = "<?php\nthrow new Exception();";
        let edits = SingleLineThrowFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
