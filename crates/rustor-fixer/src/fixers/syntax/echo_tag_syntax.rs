//! Echo tag syntax fixer
//!
//! Converts <?= to <?php echo or vice versa.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Normalizes echo tag syntax
pub struct EchoTagSyntaxFixer;

impl Fixer for EchoTagSyntaxFixer {
    fn name(&self) -> &'static str {
        "echo_tag_syntax"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "echo_tag_syntax"
    }

    fn description(&self) -> &'static str {
        "Use short echo tag <?= instead of <?php echo"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Convert <?php echo to <?=
        let echo_re = Regex::new(r"<\?php\s+echo\s+").unwrap();

        for m in echo_re.find_iter(source) {
            edits.push(edit_with_rule(
                m.start(),
                m.end(),
                "<?= ".to_string(),
                "Use short echo tag <?=".to_string(),
                "echo_tag_syntax",
            ));
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        EchoTagSyntaxFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_short_echo_unchanged() {
        let source = "<?= $var ?>";
        assert!(check(source).is_empty());
    }

    #[test]
    fn test_long_echo_to_short() {
        let source = "<?php echo $var ?>";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("<?="));
    }

    #[test]
    fn test_multiple_echoes() {
        let source = "<?php echo $a ?> text <?php echo $b ?>";
        let edits = check(source);
        assert_eq!(edits.len(), 2);
    }
}
