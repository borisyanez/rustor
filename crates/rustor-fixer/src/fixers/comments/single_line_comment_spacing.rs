//! Single line comment spacing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct SingleLineCommentSpacingFixer;

impl Fixer for SingleLineCommentSpacingFixer {
    fn name(&self) -> &'static str { "single_line_comment_spacing" }
    fn php_cs_fixer_name(&self) -> &'static str { "single_line_comment_spacing" }
    fn description(&self) -> &'static str { "Ensure space after //" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match // not followed by space (but not ///)
        let re = Regex::new(r"//([^\s/])").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let char = cap.get(1).unwrap().as_str();

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("// {}", char),
                "Add space after //".to_string(),
                "single_line_comment_spacing",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_no_space() {
        let code = "<?php\n//comment";
        let edits = SingleLineCommentSpacingFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_has_space() {
        let code = "<?php\n// comment";
        let edits = SingleLineCommentSpacingFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }

    #[test]
    fn test_triple_slash() {
        let code = "<?php\n/// docs";
        let edits = SingleLineCommentSpacingFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
