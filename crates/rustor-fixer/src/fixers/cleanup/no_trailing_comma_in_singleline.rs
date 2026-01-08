//! No trailing comma in single line

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoTrailingCommaInSinglelineFixer;

impl Fixer for NoTrailingCommaInSinglelineFixer {
    fn name(&self) -> &'static str { "no_trailing_comma_in_singleline" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_trailing_comma_in_singleline" }
    fn description(&self) -> &'static str { "No trailing comma in single line" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match trailing comma before ] or ) on same line
        let re = Regex::new(r",(\s*)([\]\)])").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let space = cap.get(1).unwrap().as_str();
            let close = cap.get(2).unwrap().as_str();

            // Only if no newline in the space
            if !space.contains('\n') {
                if is_in_string(&source[..full.start()]) { continue; }

                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    close.to_string(),
                    "Remove trailing comma in single line".to_string(),
                    "no_trailing_comma_in_singleline",
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
    fn test_trailing_comma_singleline() {
        let edits = NoTrailingCommaInSinglelineFixer.check("<?php\n$a = [1, 2,];", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_no_trailing_comma() {
        let edits = NoTrailingCommaInSinglelineFixer.check("<?php\n$a = [1, 2];", &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
