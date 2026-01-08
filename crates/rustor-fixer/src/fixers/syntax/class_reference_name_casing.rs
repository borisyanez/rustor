//! Lowercase self, static, parent references

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct ClassReferenceNameCasingFixer;

impl Fixer for ClassReferenceNameCasingFixer {
    fn name(&self) -> &'static str { "class_reference_name_casing" }
    fn php_cs_fixer_name(&self) -> &'static str { "class_reference_name_casing" }
    fn description(&self) -> &'static str { "Lowercase self, static, parent" }
    fn priority(&self) -> i32 { 40 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match uppercase variants
        let patterns = [
            (r"\b(SELF|Self)\s*::", "self::"),
            (r"\b(STATIC|Static)\s*::", "static::"),
            (r"\b(PARENT|Parent)\s*::", "parent::"),
        ];

        for (pattern, replacement) in &patterns {
            let re = Regex::new(pattern).unwrap();
            for m in re.find_iter(source) {
                if is_in_string(&source[..m.start()]) { continue; }

                edits.push(edit_with_rule(
                    m.start(), m.end(),
                    replacement.to_string(),
                    "Use lowercase class reference".to_string(),
                    "class_reference_name_casing",
                ));
            }
        }

        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let (mut s, mut d, mut p) = (false, false, '\0');
    for c in before.chars() {
        if c == '\'' && p != '\\' && !d { s = !s; }
        if c == '"' && p != '\\' && !s { d = !d; }
        p = c;
    }
    s || d
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_self_uppercase() {
        let edits = ClassReferenceNameCasingFixer.check("<?php\nSELF::foo();", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_self_lowercase() {
        let edits = ClassReferenceNameCasingFixer.check("<?php\nself::foo();", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
