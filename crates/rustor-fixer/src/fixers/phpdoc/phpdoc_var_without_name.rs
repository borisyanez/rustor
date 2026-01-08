//! Remove variable name from @var when obvious

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocVarWithoutNameFixer;

impl Fixer for PhpdocVarWithoutNameFixer {
    fn name(&self) -> &'static str { "phpdoc_var_without_name" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_var_without_name" }
    fn description(&self) -> &'static str { "Remove variable name from @var" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match /** @var Type $name */ followed by $name = ...
        let re = Regex::new(r"(/\*\*\s*@var\s+\S+)\s+(\$\w+)\s*\*/(\s*\n\s*)(\$\w+)").unwrap();

        for cap in re.captures_iter(source) {
            let var_in_doc = cap.get(2).unwrap().as_str();
            let var_after = cap.get(4).unwrap().as_str();

            // If the variable name matches, we can remove it from the doc
            if var_in_doc == var_after {
                let full = cap.get(0).unwrap();
                let prefix = cap.get(1).unwrap().as_str();
                let spacing = cap.get(3).unwrap().as_str();

                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{} */{}{}", prefix, spacing, var_after),
                    "Remove redundant variable name from @var".to_string(),
                    "phpdoc_var_without_name",
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
    fn test_var_without_name() {
        let code = "<?php
/** @var int $x */
$x = 1;";
        let edits = PhpdocVarWithoutNameFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(!edits[0].replacement.contains("$x */"));
    }
}
