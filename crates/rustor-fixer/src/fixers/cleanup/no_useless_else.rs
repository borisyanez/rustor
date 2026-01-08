//! Remove useless else after return/throw/continue/break

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NoUselessElseFixer;

impl Fixer for NoUselessElseFixer {
    fn name(&self) -> &'static str { "no_useless_else" }
    fn php_cs_fixer_name(&self) -> &'static str { "no_useless_else" }
    fn description(&self) -> &'static str { "Remove useless else blocks after early return" }
    fn priority(&self) -> i32 { 20 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        // Match: } else { ... } where if block ends with return/throw/continue/break
        // This is a simplified check - looks for pattern where else is unnecessary
        let re = Regex::new(r"(?ms)(return|throw|continue|break)[^;]*;\s*\}\s*(else\s*\{)").unwrap();

        for cap in re.captures_iter(source) {
            let else_match = cap.get(2).unwrap();
            // Find the matching closing brace for this else block
            let start = else_match.start();
            let after_else = &source[else_match.end()..];

            if let Some(close_pos) = find_matching_brace(after_else) {
                let end = else_match.end() + close_pos + 1;
                let else_body = &source[else_match.end()..else_match.end() + close_pos];
                // Replace else { body } with just body (dedented)
                let dedented = dedent_block(else_body.trim());
                edits.push(edit_with_rule(
                    start, end, dedented,
                    "Remove useless else after return".to_string(),
                    "no_useless_else",
                ));
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

fn dedent_block(s: &str) -> String {
    // Simple dedent - remove one level of indentation
    s.lines()
        .map(|line| {
            if line.starts_with("    ") {
                &line[4..]
            } else if line.starts_with('\t') {
                &line[1..]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_useless_else() {
        let code = "<?php
if ($x) {
    return 1;
} else {
    return 2;
}";
        let edits = NoUselessElseFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
