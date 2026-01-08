//! Add declare(strict_types=1) to PHP files
//!
//! This is a risky fixer because enabling strict types can cause
//! TypeError exceptions in code that was previously working with
//! type coercion.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Adds declare(strict_types=1) after the opening PHP tag
pub struct DeclareStrictTypesFixer;

impl Fixer for DeclareStrictTypesFixer {
    fn name(&self) -> &'static str {
        "declare_strict_types"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "declare_strict_types"
    }

    fn description(&self) -> &'static str {
        "Add declare(strict_types=1) at the beginning of PHP files"
    }

    fn priority(&self) -> i32 {
        5  // Low priority
    }

    fn is_risky(&self) -> bool {
        true
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Check if already has declare(strict_types=1)
        let has_strict_types = Regex::new(r"declare\s*\(\s*strict_types\s*=\s*1\s*\)")
            .unwrap()
            .is_match(source);

        if has_strict_types {
            return edits;
        }

        // Find the opening PHP tag
        let open_tag_re = Regex::new(r"<\?php\b").unwrap();

        if let Some(open_tag) = open_tag_re.find(source) {
            let line_ending = config.line_ending.as_str();

            // Insert declare(strict_types=1) after the opening tag
            let insert_pos = open_tag.end();

            // Check what comes after the opening tag
            let after_tag = &source[insert_pos..];
            let needs_newline = !after_tag.starts_with('\n') && !after_tag.starts_with('\r');

            let replacement = if needs_newline {
                format!("{line_ending}declare(strict_types=1);{line_ending}")
            } else if after_tag.starts_with("\r\n") {
                format!("{line_ending}declare(strict_types=1);")
            } else if after_tag.starts_with('\n') {
                format!("{line_ending}declare(strict_types=1);")
            } else {
                format!("{line_ending}declare(strict_types=1);{line_ending}")
            };

            edits.push(edit_with_rule(
                insert_pos,
                insert_pos,
                replacement,
                "Add declare(strict_types=1) for strict type checking".to_string(),
                "declare_strict_types",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LineEnding;

    fn check(source: &str) -> Vec<Edit> {
        DeclareStrictTypesFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        })
    }

    #[test]
    fn test_already_has_strict_types() {
        let source = "<?php\ndeclare(strict_types=1);";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_already_has_strict_types_with_space() {
        let source = "<?php\ndeclare( strict_types = 1 );";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_adds_strict_types() {
        let source = "<?php\nnamespace App;";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("declare(strict_types=1)"));
    }

    #[test]
    fn test_is_risky() {
        assert!(DeclareStrictTypesFixer.is_risky());
    }
}
