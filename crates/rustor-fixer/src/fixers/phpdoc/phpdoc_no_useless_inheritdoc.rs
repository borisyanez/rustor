//! Remove useless @inheritdoc

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct PhpdocNoUselessInheritdocFixer;

impl Fixer for PhpdocNoUselessInheritdocFixer {
    fn name(&self) -> &'static str { "phpdoc_no_useless_inheritdoc" }
    fn php_cs_fixer_name(&self) -> &'static str { "phpdoc_no_useless_inheritdoc" }
    fn description(&self) -> &'static str { "Remove useless @inheritdoc" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match standalone @inheritdoc PHPDoc (nothing else useful)
        let re = Regex::new(r"/\*\*\s*\n?\s*\*?\s*@?(?:\{@inheritdoc\}|inheritdoc)\s*\n?\s*\*/").unwrap();

        for m in re.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(), m.end(),
                String::new(),
                "Remove useless @inheritdoc".to_string(),
                "phpdoc_no_useless_inheritdoc",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_useless_inheritdoc() {
        let code = "<?php
/**
 * {@inheritdoc}
 */";
        let edits = PhpdocNoUselessInheritdocFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_inheritdoc_with_other_unchanged() {
        let code = "<?php
/**
 * {@inheritdoc}
 * @param int $x
 */";
        let edits = PhpdocNoUselessInheritdocFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
