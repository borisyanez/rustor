//! Control structure continuation position fixer
//!
//! Ensures `} else {`, `} elseif {`, `} catch {`, `} finally {` are on the same line.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures control structure continuations are on the same line as closing brace
pub struct ControlStructureContinuationPositionFixer;

impl Fixer for ControlStructureContinuationPositionFixer {
    fn name(&self) -> &'static str {
        "control_structure_continuation_position"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "control_structure_continuation_position"
    }

    fn description(&self) -> &'static str {
        "Ensure } else {, } catch {, etc. are on the same line"
    }

    fn priority(&self) -> i32 {
        35
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match `}\n  else` or `}\n  elseif` or `}\n  catch` or `}\n  finally`
        let re = Regex::new(r"\}(\s*\n\s*)(else\s*if|elseif|else|catch|finally)\b").unwrap();

        for cap in re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let whitespace = cap.get(1).unwrap();
            let keyword = cap.get(2).unwrap().as_str();

            // Skip if in string
            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Replace newline+whitespace with single space
            edits.push(edit_with_rule(
                whitespace.start(),
                whitespace.end(),
                " ".to_string(),
                format!("'{}' should be on same line as closing brace", keyword),
                "control_structure_continuation_position",
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

    fn check(source: &str) -> Vec<Edit> {
        ControlStructureContinuationPositionFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nif ($a) {\n} else {\n}";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_else_on_new_line() {
        let source = "<?php\nif ($a) {\n}\nelse {\n}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, " ");
    }

    #[test]
    fn test_elseif_on_new_line() {
        let source = "<?php\nif ($a) {\n}\nelseif ($b) {\n}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_catch_on_new_line() {
        let source = "<?php\ntry {\n}\ncatch (Exception $e) {\n}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_finally_on_new_line() {
        let source = "<?php\ntry {\n} catch (Exception $e) {\n}\nfinally {\n}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_with_indentation() {
        let source = "<?php\nif ($a) {\n}    \n    else {\n}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_apply_edit() {
        let source = "<?php\nif ($a) {\n}\nelseif ($b) {\n}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);

        // Check the edit details
        let edit = &edits[0];
        let start = edit.span.start.offset as usize;
        let end = edit.span.end.offset as usize;

        println!("Source length: {}", source.len());
        println!("Edit span: {}..{}", start, end);
        println!("Text at span: {:?}", &source[start..end]);
        println!("Replacement: {:?}", edit.replacement);

        // Apply the edit
        let result = rustor_core::apply_edits(source, &edits).unwrap();
        println!("Result: {:?}", result);

        // Should now be on same line
        assert!(result.contains("} elseif"), "Expected '}} elseif' but got: {}", result);
    }
}
