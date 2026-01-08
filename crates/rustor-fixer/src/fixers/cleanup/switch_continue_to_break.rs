//! Replace continue in switch with break

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct SwitchContinueToBreakFixer;

impl Fixer for SwitchContinueToBreakFixer {
    fn name(&self) -> &'static str { "switch_continue_to_break" }
    fn php_cs_fixer_name(&self) -> &'static str { "switch_continue_to_break" }
    fn description(&self) -> &'static str { "Replace continue in switch with break" }
    fn priority(&self) -> i32 { 20 }
    fn is_risky(&self) -> bool { true }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Find switch statements and look for continue; inside them
        let switch_re = Regex::new(r"(?ms)\bswitch\s*\([^)]+\)\s*\{").unwrap();

        for switch_match in switch_re.find_iter(source) {
            let start = switch_match.end();
            if let Some(end) = find_matching_brace(&source[start..]) {
                let switch_body = &source[start..start + end];
                let body_start = start;

                // Track nesting of for/foreach/while loops
                // Only replace continue; that refers to switch (continue 1 or just continue)
                let continue_re = Regex::new(r"\bcontinue\s*;").unwrap();

                for m in continue_re.find_iter(switch_body) {
                    // Check if we're inside a nested loop
                    let before = &switch_body[..m.start()];
                    if !is_in_nested_loop(before) {
                        edits.push(edit_with_rule(
                            body_start + m.start(), body_start + m.end(),
                            "break;".to_string(),
                            "Replace continue with break in switch".to_string(),
                            "switch_continue_to_break",
                        ));
                    }
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

fn is_in_nested_loop(before: &str) -> bool {
    // Simple heuristic: count loop starts vs closes
    let loop_re = Regex::new(r"\b(for|foreach|while|do)\b").unwrap();
    let close_re = Regex::new(r"\}").unwrap();

    let mut depth = 0;
    let mut pos = 0;

    // Interleave loop keywords and closing braces
    let loops: Vec<_> = loop_re.find_iter(before).map(|m| (m.start(), true)).collect();
    let closes: Vec<_> = close_re.find_iter(before).map(|m| (m.start(), false)).collect();

    let mut events: Vec<_> = loops.into_iter().chain(closes).collect();
    events.sort_by_key(|e| e.0);

    for (_, is_loop) in events {
        if is_loop {
            depth += 1;
        } else if depth > 0 {
            depth -= 1;
        }
    }

    depth > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_continue_in_switch() {
        let code = "<?php
switch ($x) {
    case 1:
        continue;
}";
        let edits = SwitchContinueToBreakFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        assert_eq!(edits[0].replacement, "break;");
    }

    #[test]
    fn test_continue_in_loop_in_switch() {
        let code = "<?php
switch ($x) {
    case 1:
        foreach ($arr as $v) {
            continue;
        }
}";
        let edits = SwitchContinueToBreakFixer.check(code, &FixerConfig::default());
        // Should NOT replace continue inside the foreach
        assert!(edits.is_empty());
    }
}
