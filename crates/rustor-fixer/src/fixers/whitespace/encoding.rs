//! Ensure UTF-8 encoding without BOM

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures file is UTF-8 encoded without BOM
pub struct EncodingFixer;

impl Fixer for EncodingFixer {
    fn name(&self) -> &'static str {
        "encoding"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "encoding"
    }

    fn description(&self) -> &'static str {
        "Ensure UTF-8 encoding without BOM"
    }

    fn priority(&self) -> i32 {
        100 // Highest priority - should run first
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let bytes = source.as_bytes();

        // Check for UTF-8 BOM (EF BB BF)
        if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
            vec![edit_with_rule(
                0,
                3,
                String::new(),
                "Remove UTF-8 BOM".to_string(),
                "encoding",
            )]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        EncodingFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_no_bom() {
        let edits = check("<?php\n$a = 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_with_utf8_bom() {
        // UTF-8 BOM + PHP code
        let source = "\u{FEFF}<?php\n$a = 1;\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start_offset(), 0);
        assert_eq!(edits[0].end_offset(), 3);
        assert_eq!(edits[0].replacement, "");
    }

    #[test]
    fn test_short_file() {
        let edits = check("<?");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_file() {
        let edits = check("");
        assert!(edits.is_empty());
    }
}
