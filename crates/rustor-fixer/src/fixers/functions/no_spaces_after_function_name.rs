//! No spaces after function name fixer
//!
//! Removes whitespace between function name and opening parenthesis.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes whitespace between function name and opening parenthesis
pub struct NoSpacesAfterFunctionNameFixer;

impl Fixer for NoSpacesAfterFunctionNameFixer {
    fn name(&self) -> &'static str {
        "no_spaces_after_function_name"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_spaces_after_function_name"
    }

    fn description(&self) -> &'static str {
        "Remove whitespace between function name and opening parenthesis"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match function calls with space before (
        // foo () -> foo()
        // $this->bar () -> $this->bar()
        let call_re = Regex::new(r"\b([a-zA-Z_]\w*)\s+\(").unwrap();

        for cap in call_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let func_name = cap.get(1).unwrap().as_str();

            // Skip if in string
            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Skip if in comment
            if is_in_comment(&source[..full_match.start()]) {
                continue;
            }

            // Skip PHP keywords that require a space
            let keywords = [
                "if", "elseif", "else", "while", "for", "foreach", "switch",
                "catch", "match", "fn", "function", "class", "interface",
                "trait", "enum", "new", "return", "yield", "throw", "echo",
                "print", "include", "include_once", "require", "require_once",
                "use", "namespace", "extends", "implements", "instanceof",
                "array", "list", "isset", "unset", "empty", "eval", "exit", "die",
            ];

            if keywords.contains(&func_name.to_lowercase().as_str()) {
                continue;
            }

            // Get position of the space to remove
            let name_end = cap.get(1).unwrap().end();
            let paren_start = full_match.end() - 1;

            edits.push(edit_with_rule(
                name_end,
                paren_start,
                "".to_string(),
                format!("Remove space between '{}' and '('", func_name),
                "no_spaces_after_function_name",
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

fn is_in_comment(before: &str) -> bool {
    if let Some(last_line_start) = before.rfind('\n') {
        let last_line = &before[last_line_start..];
        if last_line.contains("//") || last_line.contains('#') {
            return true;
        }
    } else if before.contains("//") {
        return true;
    }

    let open_count = before.matches("/*").count();
    let close_count = before.matches("*/").count();
    open_count > close_count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        NoSpacesAfterFunctionNameFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nfoo();";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_function_call_with_space() {
        let source = "<?php\nfoo ();";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "");
    }

    #[test]
    fn test_method_call_with_space() {
        let source = "<?php\n$obj->bar ();";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_if_keyword() {
        let source = "<?php\nif ($a) {}";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_foreach_keyword() {
        let source = "<?php\nforeach ($arr as $item) {}";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_function_declaration() {
        let source = "<?php\nfunction foo() {}";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_spaces() {
        let source = "<?php\nfoo   ();";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'foo ()';";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
