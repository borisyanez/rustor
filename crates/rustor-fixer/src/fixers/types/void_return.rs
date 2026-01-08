//! Add void return type to functions that don't return

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct VoidReturnFixer;

impl Fixer for VoidReturnFixer {
    fn name(&self) -> &'static str { "void_return" }
    fn php_cs_fixer_name(&self) -> &'static str { "void_return" }
    fn description(&self) -> &'static str { "Add void return type" }
    fn priority(&self) -> i32 { 20 }
    fn is_risky(&self) -> bool { true }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match function without return type that has no return statement
        // This is a simplified check - full analysis would need AST
        let func_re = Regex::new(r"(function\s+\w+\s*\([^)]*\))\s*\{").unwrap();

        for cap in func_re.captures_iter(source) {
            let sig = cap.get(1).unwrap();
            let sig_text = sig.as_str();

            // Skip if already has return type
            if sig_text.contains(':') { continue; }

            // Find function body
            let func_start = cap.get(0).unwrap().end();
            if let Some(body_end) = find_matching_brace(&source[func_start..]) {
                let body = &source[func_start..func_start + body_end];

                // Check for return statements with values
                let return_re = Regex::new(r"\breturn\s+[^;]").unwrap();
                if !return_re.is_match(body) {
                    // No return with value - suggest void
                    edits.push(edit_with_rule(
                        sig.start(), sig.end(),
                        format!("{}: void", sig_text),
                        "Add void return type".to_string(),
                        "void_return",
                    ));
                }
            }
        }

        edits
    }
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 1;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 { return Some(i); }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_void_return() {
        let code = "<?php
function foo() {
    echo 'hi';
}";
        let edits = VoidReturnFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(": void"));
    }

    #[test]
    fn test_with_return_unchanged() {
        let code = "<?php
function foo() {
    return 1;
}";
        let edits = VoidReturnFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
