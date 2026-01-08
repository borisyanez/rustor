//! Single line @var PHPDoc spacing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocSingleLineVarSpacingFixer;

impl Fixer for PhpdocSingleLineVarSpacingFixer {
    fn name(&self) -> &'static str { "phpdoc_single_line_var_spacing" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_single_line_var_spacing" }
    fn description(&self) -> &'static str { "Fix spacing in single line @var PHPDoc" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        // Match: /** @var Type $var */ with inconsistent spacing
        let re = Regex::new(r"/\*\*\s*@var\s+(\S+)\s+(\$\w+)\s*\*/").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let type_hint = cap.get(1).unwrap().as_str();
            let var = cap.get(2).unwrap().as_str();

            let expected = format!("/** @var {} {} */", type_hint, var);
            if full.as_str() != expected {
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    expected,
                    "Fix @var spacing".to_string(),
                    "phpdoc_single_line_var_spacing",
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
    fn test_var_spacing() {
        let edits = PhpdocSingleLineVarSpacingFixer.check("<?php\n/**  @var  int  $x  */", &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "/** @var int $x */");
    }
}
