//! Fix method chaining indentation

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures method chains have proper indentation
pub struct MethodChainingIndentationFixer;

impl Fixer for MethodChainingIndentationFixer {
    fn name(&self) -> &'static str {
        "method_chaining_indentation"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "method_chaining_indentation"
    }

    fn description(&self) -> &'static str {
        "Fix method chaining indentation"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let indent_str = config.indent.as_str();

        // Match method chains where -> is at the start of a new line
        // The line should be indented relative to the first line
        let chain_re = Regex::new(r"(?m)^([ \t]*)(->)(\w+)").unwrap();

        let lines: Vec<&str> = source.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            // Check if this line starts with whitespace + ->
            if let Some(cap) = chain_re.captures(line) {
                let current_indent = cap.get(1).unwrap().as_str();
                let arrow = cap.get(2).unwrap();
                let method = cap.get(3).unwrap().as_str();

                // Find the base indent (from the line that started the chain)
                if let Some(base_indent) = find_chain_base_indent(&lines, line_idx) {
                    // Expected indent is base + one level
                    let expected_indent = format!("{}{}", base_indent, indent_str);

                    if current_indent != expected_indent {
                        // Calculate the position in the source
                        let line_start = lines[..line_idx]
                            .iter()
                            .map(|l| l.len() + 1) // +1 for newline
                            .sum::<usize>();

                        let match_end = line_start + arrow.end() + method.len();

                        edits.push(edit_with_rule(
                            line_start,
                            match_end,
                            format!("{}->{}", expected_indent, method),
                            "Fix method chain indentation".to_string(),
                            "method_chaining_indentation",
                        ));
                    }
                }
            }
        }

        edits
    }
}

fn find_chain_base_indent(lines: &[&str], current_line: usize) -> Option<String> {
    // Look backwards for the line that started this chain
    // It's the first line that doesn't start with whitespace + ->

    for i in (0..current_line).rev() {
        let line = lines[i].trim_start();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // If this line doesn't start with ->, it's our base
        if !line.starts_with("->") {
            // Get the indent of this line
            let indent_len = lines[i].len() - lines[i].trim_start().len();
            return Some(lines[i][..indent_len].to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LineEnding, IndentStyle};

    fn check(source: &str) -> Vec<Edit> {
        MethodChainingIndentationFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            indent: IndentStyle::Spaces(4),
            ..Default::default()
        })
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\n$a\n    ->foo()\n    ->bar();\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_wrong_indent() {
        let source = "<?php\n$a\n->foo();\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("    ->foo"));
    }

    #[test]
    fn test_multiple_chains() {
        let source = "<?php\n$a\n->foo()\n->bar();\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_find_base_indent() {
        let lines = vec!["<?php", "$a = $b", "    ->foo()", "    ->bar();"];
        let base = find_chain_base_indent(&lines, 2);
        assert_eq!(base, Some("".to_string()));
    }

    #[test]
    fn test_indented_base() {
        let lines = vec!["<?php", "    $a = $b", "        ->foo()"];
        let base = find_chain_base_indent(&lines, 2);
        assert_eq!(base, Some("    ".to_string()));
    }
}
