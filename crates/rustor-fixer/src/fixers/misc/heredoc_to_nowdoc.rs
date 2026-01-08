//! Convert heredoc to nowdoc when no variables

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct HeredocToNowdocFixer;

impl Fixer for HeredocToNowdocFixer {
    fn name(&self) -> &'static str { "heredoc_to_nowdoc" }
    fn php_cs_fixer_name(&self) -> &'static str { "heredoc_to_nowdoc" }
    fn description(&self) -> &'static str { "Convert heredoc to nowdoc when possible" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Find heredoc start: <<<LABEL (not already nowdoc <<<'LABEL')
        let start_re = Regex::new(r"<<<([A-Za-z_][A-Za-z0-9_]*)\n").unwrap();

        for cap in start_re.captures_iter(source) {
            let label = cap.get(1).unwrap().as_str();
            let start = cap.get(0).unwrap().start();
            let content_start = cap.get(0).unwrap().end();

            // Find the matching end label
            let end_pattern = format!("\n{}", label);
            if let Some(rel_end) = source[content_start..].find(&end_pattern) {
                let content = &source[content_start..content_start + rel_end];

                // Check if content has no variables
                if !content.contains('$') && !content.contains('{') {
                    let full_end = content_start + rel_end + end_pattern.len();
                    edits.push(edit_with_rule(
                        start, full_end,
                        format!("<<<'{}'\n{}\n{}", label, content, label),
                        "Convert heredoc to nowdoc".to_string(),
                        "heredoc_to_nowdoc",
                    ));
                }
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_heredoc_to_nowdoc() {
        let code = "<?php
$a = <<<EOF
Hello World
EOF;";
        let edits = HeredocToNowdocFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains("<<<'EOF'"));
    }

    #[test]
    fn test_heredoc_with_var_unchanged() {
        let code = "<?php
$a = <<<EOF
Hello $name
EOF;";
        let edits = HeredocToNowdocFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
