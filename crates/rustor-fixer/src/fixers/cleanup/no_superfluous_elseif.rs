//! Convert elseif to else if

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoSuperfluousElseifFixer;

impl Fixer for NoSuperfluousElseifFixer {
    fn name(&self) -> &'static str { "no_superfluous_elseif" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_superfluous_elseif" }
    fn description(&self) -> &'static str { "Remove superfluous elseif after return" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match elseif after a block that ends with return/throw/continue/break
        let re = Regex::new(r"(?ms)(return|throw|continue|break)[^;]*;\s*\}\s*(elseif)").unwrap();

        for cap in re.captures_iter(source) {
            let elseif = cap.get(2).unwrap();

            edits.push(edit_with_rule(
                elseif.start(), elseif.end(),
                "if".to_string(),
                "Convert superfluous elseif to if".to_string(),
                "no_superfluous_elseif",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_superfluous_elseif() {
        let code = "<?php
if ($a) {
    return 1;
} elseif ($b) {
    return 2;
}";
        let edits = NoSuperfluousElseifFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "if");
    }
}
