//! Backtick to shell_exec fixer

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Converts backtick operator to shell_exec()
pub struct BacktickToShellExecFixer;

impl Fixer for BacktickToShellExecFixer {
    fn name(&self) -> &'static str { "backtick_to_shell_exec" }
    fn php_cs_fixer_name(&self) -> &'static str { "backtick_to_shell_exec" }
    fn description(&self) -> &'static str { "Convert backtick operator to shell_exec()" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let re = Regex::new(r"`([^`]*)`").unwrap();

        for cap in re.captures_iter(source) {
            let full = cap.get(0).unwrap();
            let cmd = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full.start()]) { continue; }

            edits.push(edit_with_rule(
                full.start(), full.end(),
                format!("shell_exec('{}')", cmd.replace('\'', "\\'")),
                "Use shell_exec() instead of backticks".to_string(),
                "backtick_to_shell_exec",
            ));
        }
        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';
    for c in before.chars() {
        if c == '\'' && prev != '\\' && !in_double { in_single = !in_single; }
        if c == '"' && prev != '\\' && !in_single { in_double = !in_double; }
        prev = c;
    }
    in_single || in_double
}

#[cfg(test)]
mod tests {
    use super::*;
    fn check(source: &str) -> Vec<Edit> { BacktickToShellExecFixer.check(source, &FixerConfig::default()) }

    #[test]
    fn test_backtick() {
        let source = "<?php\n$a = `ls -la`;";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("shell_exec"));
    }
}
