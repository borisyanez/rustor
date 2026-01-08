//! Single space around construct

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct SingleSpaceAroundConstructFixer;

impl Fixer for SingleSpaceAroundConstructFixer {
    fn name(&self) -> &'static str { "single_space_around_construct" }
    fn php_cs_fixer_name(&self) -> &'static str { "single_space_around_construct" }
    fn description(&self) -> &'static str { "Single space around constructs" }
    fn priority(&self) -> i32 { 30 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Control structures that need single space before (
        let constructs = ["if", "elseif", "while", "for", "foreach", "switch", "catch"];

        for construct in constructs {
            // Multiple spaces between construct and (
            let re = Regex::new(&format!(r"\b({})\s{{2,}}\(", construct)).unwrap();
            for cap in re.captures_iter(source) {
                let full = cap.get(0).unwrap();
                let kw = cap.get(1).unwrap().as_str();
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{} (", kw),
                    format!("Single space after {}", construct),
                    "single_space_around_construct",
                ));
            }

            // No space between construct and (
            let re = Regex::new(&format!(r"\b({})\(", construct)).unwrap();
            for cap in re.captures_iter(source) {
                let full = cap.get(0).unwrap();
                let kw = cap.get(1).unwrap().as_str();
                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{} (", kw),
                    format!("Add space after {}", construct),
                    "single_space_around_construct",
                ));
            }
        }

        // Control structures need single space before opening brace
        // Match ){  (closing paren immediately followed by opening brace)
        // Use a simpler approach that works with nested parentheses
        let brace_re = Regex::new(r"\)\{").unwrap();
        for cap in brace_re.captures_iter(source) {
            let full = cap.get(0).unwrap();

            // Skip if in string
            if is_in_string(&source[..full.start()]) {
                continue;
            }

            // Replace ){ with ) {
            edits.push(edit_with_rule(
                full.start(),
                full.end(),
                ") {".to_string(),
                "Add space before opening brace".to_string(),
                "single_space_around_construct",
            ));
        }

        // Also handle else{ (no parentheses)
        let else_brace_re = Regex::new(r"\belse\{").unwrap();
        for cap in else_brace_re.captures_iter(source) {
            let full = cap.get(0).unwrap();

            // Skip if in string
            if is_in_string(&source[..full.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full.start(),
                full.end(),
                "else {".to_string(),
                "Add space before opening brace".to_string(),
                "single_space_around_construct",
            ));
        }

        // Handle try{
        let try_brace_re = Regex::new(r"\btry\{").unwrap();
        for cap in try_brace_re.captures_iter(source) {
            let full = cap.get(0).unwrap();

            if is_in_string(&source[..full.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full.start(),
                full.end(),
                "try {".to_string(),
                "Add space before opening brace".to_string(),
                "single_space_around_construct",
            ));
        }

        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        if c == '\'' && prev_char != '\\' && !in_double_quote {
            in_single_quote = !in_single_quote;
        }
        if c == '"' && prev_char != '\\' && !in_single_quote {
            in_double_quote = !in_double_quote;
        }
        prev_char = c;
    }

    in_single_quote || in_double_quote
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_no_space() {
        let code = "<?php\nif(true) {}";
        let edits = SingleSpaceAroundConstructFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_multiple_spaces() {
        let code = "<?php\nif  (true) {}";
        let edits = SingleSpaceAroundConstructFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_single_space() {
        let code = "<?php\nif (true) {}";
        let edits = SingleSpaceAroundConstructFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }

    #[test]
    fn test_no_space_before_brace() {
        let code = "<?php\nif (true){}";
        let edits = SingleSpaceAroundConstructFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        // Should add space before {
        let edit = edits.iter().find(|e| e.message.contains("brace")).unwrap();
        assert_eq!(edit.replacement, ") {");
    }

    #[test]
    fn test_nested_parens_no_space() {
        let code = "<?php\nif (!is_array($input)){}";
        let edits = SingleSpaceAroundConstructFixer.check(code, &FixerConfig::default());
        assert!(!edits.is_empty());
        let edit = edits.iter().find(|e| e.message.contains("brace")).unwrap();
        assert_eq!(edit.replacement, ") {");
    }

    #[test]
    fn test_correct_brace_spacing() {
        let code = "<?php\nif (true) {}";
        let edits = SingleSpaceAroundConstructFixer.check(code, &FixerConfig::default());
        assert!(edits.is_empty());
    }
}
