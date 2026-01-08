//! Ensure control structures have braces

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures control structures use braces
pub struct ControlStructureBracesFixer;

impl Fixer for ControlStructureBracesFixer {
    fn name(&self) -> &'static str {
        "control_structure_braces"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "control_structure_braces"
    }

    fn description(&self) -> &'static str {
        "Ensure control structures use braces"
    }

    fn priority(&self) -> i32 {
        35
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Match if statements without braces
        // if ($cond) statement; -> if ($cond) { statement; }
        let if_re = Regex::new(r"(?m)\b(if\s*\([^)]+\))\s*([^{\s][^;\n]*;)").unwrap();

        for cap in if_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let condition = cap.get(1).unwrap().as_str();
            let statement = cap.get(2).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Skip if this is part of an else if
            let before = &source[..full_match.start()];
            let trimmed = before.trim_end();
            if trimmed.ends_with("else") {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{} {{{}{}{}}}", condition, line_ending, statement, line_ending),
                "Control structure must use braces".to_string(),
                "control_structure_braces",
            ));
        }

        // Match else without braces (but not else if)
        // else statement; -> else { statement; }
        let else_re = Regex::new(r"(?m)\belse\s+([^{\s][^;\n]*;)").unwrap();

        for cap in else_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let statement = cap.get(1).unwrap().as_str();

            // Skip if this is else if (manually check since we can't use look-ahead)
            if statement.trim_start().starts_with("if") {
                continue;
            }

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Check not already handled
            let already_edited = edits.iter().any(|e| {
                e.start_offset() <= full_match.start() && e.end_offset() >= full_match.end()
            });

            if !already_edited {
                edits.push(edit_with_rule(
                    full_match.start(),
                    full_match.end(),
                    format!("else {{{}{}{}}}", line_ending, statement, line_ending),
                    "Control structure must use braces".to_string(),
                    "control_structure_braces",
                ));
            }
        }

        // Match while without braces
        let while_re = Regex::new(r"(?m)\b(while\s*\([^)]+\))\s*([^{\s][^;\n]*;)").unwrap();

        for cap in while_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let condition = cap.get(1).unwrap().as_str();
            let statement = cap.get(2).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Skip do-while
            let before = &source[..full_match.start()];
            let trimmed = before.trim_end();
            if trimmed.ends_with('}') {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{} {{{}{}{}}}", condition, line_ending, statement, line_ending),
                "Control structure must use braces".to_string(),
                "control_structure_braces",
            ));
        }

        // Match for without braces
        let for_re = Regex::new(r"(?m)\b(for\s*\([^)]+\))\s*([^{\s][^;\n]*;)").unwrap();

        for cap in for_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let condition = cap.get(1).unwrap().as_str();
            let statement = cap.get(2).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{} {{{}{}{}}}", condition, line_ending, statement, line_ending),
                "Control structure must use braces".to_string(),
                "control_structure_braces",
            ));
        }

        // Match foreach without braces
        let foreach_re = Regex::new(r"(?m)\b(foreach\s*\([^)]+\))\s*([^{\s][^;\n]*;)").unwrap();

        for cap in foreach_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let condition = cap.get(1).unwrap().as_str();
            let statement = cap.get(2).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{} {{{}{}{}}}", condition, line_ending, statement, line_ending),
                "Control structure must use braces".to_string(),
                "control_structure_braces",
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
    use crate::config::LineEnding;

    fn check(source: &str) -> Vec<Edit> {
        ControlStructureBracesFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        })
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nif ($a) { foo(); }\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_if_without_braces() {
        let source = "<?php\nif ($a) foo();";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("if ($a) {"));
        assert!(edits[0].replacement.contains("foo();"));
    }

    #[test]
    fn test_else_without_braces() {
        let source = "<?php\nif ($a) { } else foo();";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("else {"));
    }

    #[test]
    fn test_while_without_braces() {
        let source = "<?php\nwhile ($a) foo();";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("while ($a) {"));
    }

    #[test]
    fn test_for_without_braces() {
        let source = "<?php\nfor ($i = 0; $i < 10; $i++) foo();";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("for ($i = 0; $i < 10; $i++) {"));
    }

    #[test]
    fn test_foreach_without_braces() {
        let source = "<?php\nforeach ($arr as $item) foo();";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("foreach ($arr as $item) {"));
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'if ($a) foo();';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
