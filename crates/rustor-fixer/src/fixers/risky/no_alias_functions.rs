//! Replace PHP function aliases with their canonical names
//!
//! This is a risky fixer because some function aliases may have slightly
//! different behavior or parameter signatures.

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Replaces function aliases with their canonical function names
pub struct NoAliasFunctionsFixer;

impl Fixer for NoAliasFunctionsFixer {
    fn name(&self) -> &'static str {
        "no_alias_functions"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_alias_functions"
    }

    fn description(&self) -> &'static str {
        "Replace function aliases with their canonical names"
    }

    fn priority(&self) -> i32 {
        5  // Low priority
    }

    fn is_risky(&self) -> bool {
        true
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Build regex pattern for all function aliases
        let aliases: Vec<(&str, &str)> = vec![
            // String functions
            ("chop", "rtrim"),
            ("close", "closedir"),
            ("doubleval", "floatval"),
            ("fputs", "fwrite"),
            ("ini_alter", "ini_set"),
            ("is_double", "is_float"),
            ("is_integer", "is_int"),
            ("is_long", "is_int"),
            ("is_real", "is_float"),
            ("is_writeable", "is_writable"),
            ("join", "implode"),
            ("key_exists", "array_key_exists"),
            ("magic_quotes_runtime", "set_magic_quotes_runtime"),
            ("pos", "current"),
            ("show_source", "highlight_file"),
            ("sizeof", "count"),
            ("strchr", "strstr"),
            ("user_error", "trigger_error"),
            // mbstring aliases
            ("mbereg", "mb_ereg"),
            ("mbereg_match", "mb_ereg_match"),
            ("mbereg_replace", "mb_ereg_replace"),
            ("mbereg_search", "mb_ereg_search"),
            ("mbereg_search_getpos", "mb_ereg_search_getpos"),
            ("mbereg_search_getregs", "mb_ereg_search_getregs"),
            ("mbereg_search_init", "mb_ereg_search_init"),
            ("mbereg_search_pos", "mb_ereg_search_pos"),
            ("mbereg_search_regs", "mb_ereg_search_regs"),
            ("mbereg_search_setpos", "mb_ereg_search_setpos"),
            ("mberegi", "mb_eregi"),
            ("mberegi_replace", "mb_eregi_replace"),
            ("mbregex_encoding", "mb_regex_encoding"),
            ("mbsplit", "mb_split"),
        ];

        for (alias, canonical) in aliases {
            // Match function call with word boundaries
            let pattern = format!(r"\b{}\s*\(", regex::escape(alias));
            let re = Regex::new(&pattern).unwrap();

            for m in re.find_iter(source) {
                // Skip if in string
                if is_in_string(&source[..m.start()]) {
                    continue;
                }

                // Skip if in comment
                if is_in_comment(&source[..m.start()]) {
                    continue;
                }

                // Skip if it's a method call (preceded by -> or ::)
                let before = &source[..m.start()];
                let trimmed = before.trim_end();
                if trimmed.ends_with("->") || trimmed.ends_with("::") {
                    continue;
                }

                // Get the function name part (without parenthesis)
                let func_end = m.as_str().find('(').unwrap_or(m.len());
                let func_with_space = &m.as_str()[..func_end];
                let has_space = func_with_space.ends_with(' ');

                let replacement = format!("{}{}", canonical, if has_space { " (" } else { "(" });

                edits.push(edit_with_rule(
                    m.start(),
                    m.end(),
                    replacement,
                    format!("Replace deprecated '{}' with '{}'", alias, canonical),
                    "no_alias_functions",
                ));
            }
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
    // Check for single-line comment
    if let Some(last_line_start) = before.rfind('\n') {
        let last_line = &before[last_line_start..];
        if last_line.contains("//") || last_line.contains('#') {
            return true;
        }
    } else if before.contains("//") || before.contains('#') {
        return true;
    }

    // Check for multi-line comment (not closed)
    let open_count = before.matches("/*").count();
    let close_count = before.matches("*/").count();
    open_count > close_count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        NoAliasFunctionsFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_canonical_unchanged() {
        let source = "<?php\n$a = count($arr);";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_sizeof_to_count() {
        let source = "<?php\n$a = sizeof($arr);";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("count"));
    }

    #[test]
    fn test_join_to_implode() {
        let source = "<?php\n$str = join(',', $arr);";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("implode"));
    }

    #[test]
    fn test_is_integer_to_is_int() {
        let source = "<?php\nif (is_integer($x)) {}";
        let edits = check(source);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("is_int"));
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php\n$obj->sizeof();";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'sizeof($arr)';";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_is_risky() {
        assert!(NoAliasFunctionsFixer.is_risky());
    }
}
