//! Lowercase PHP keywords

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// PHP keywords that should be lowercase
const PHP_KEYWORDS: &[&str] = &[
    "abstract", "and", "array", "as", "break", "callable", "case", "catch",
    "class", "clone", "const", "continue", "declare", "default", "do", "echo",
    "else", "elseif", "empty", "enddeclare", "endfor", "endforeach", "endif",
    "endswitch", "endwhile", "enum", "eval", "exit", "extends", "final",
    "finally", "fn", "for", "foreach", "function", "global", "goto", "if",
    "implements", "include", "include_once", "instanceof", "insteadof",
    "interface", "isset", "list", "match", "namespace", "new", "or", "print",
    "private", "protected", "public", "readonly", "require", "require_once",
    "return", "static", "switch", "throw", "trait", "try", "unset", "use",
    "var", "while", "xor", "yield", "yield from",
    // Type keywords
    "bool", "float", "int", "string", "void", "mixed", "never", "object",
    "iterable", "null", "false", "true", "self", "parent",
];

/// Ensures PHP keywords are lowercase
pub struct LowercaseKeywordsFixer;

impl Fixer for LowercaseKeywordsFixer {
    fn name(&self) -> &'static str {
        "lowercase_keywords"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "lowercase_keywords"
    }

    fn description(&self) -> &'static str {
        "PHP keywords must be lowercase"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Build regex pattern for all keywords (case insensitive)
        for keyword in PHP_KEYWORDS {
            // Word boundary pattern
            let pattern = format!(r"(?i)\b({})\b", regex::escape(keyword));
            let re = Regex::new(&pattern).unwrap();

            for cap in re.captures_iter(source) {
                let matched = cap.get(1).unwrap();
                let matched_str = matched.as_str();

                // Skip if already lowercase
                if matched_str == *keyword {
                    continue;
                }

                // Skip if inside a string or comment (simple heuristic)
                let before = &source[..matched.start()];
                if is_in_string_or_comment(before) {
                    continue;
                }

                edits.push(edit_with_rule(
                    matched.start(),
                    matched.end(),
                    keyword.to_string(),
                    format!("Lowercase keyword '{}'", matched_str),
                    "lowercase_keywords",
                ));
            }
        }

        edits
    }
}

/// Simple heuristic to check if position might be in string or comment
fn is_in_string_or_comment(before: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        // Check for line comment start
        if !in_single_quote && !in_double_quote && !in_block_comment {
            if c == '/' && prev_char == '/' {
                in_line_comment = true;
            }
            if c == '#' {
                in_line_comment = true;
            }
        }

        // Check for block comment
        if !in_single_quote && !in_double_quote && !in_line_comment {
            if c == '*' && prev_char == '/' {
                in_block_comment = true;
            }
            if c == '/' && prev_char == '*' && in_block_comment {
                in_block_comment = false;
            }
        }

        // Reset line comment at newline
        if c == '\n' {
            in_line_comment = false;
        }

        // Track string state
        if !in_line_comment && !in_block_comment {
            if c == '\'' && prev_char != '\\' && !in_double_quote {
                in_single_quote = !in_single_quote;
            }
            if c == '"' && prev_char != '\\' && !in_single_quote {
                in_double_quote = !in_double_quote;
            }
        }

        prev_char = c;
    }

    in_single_quote || in_double_quote || in_line_comment || in_block_comment
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        LowercaseKeywordsFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_lowercase_unchanged() {
        let edits = check("<?php\nif ($a) { return true; }\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_uppercase_if() {
        let source = "<?php\nIF ($a) { }\n";
        let edits = check(source);

        assert!(!edits.is_empty());
        assert!(edits.iter().any(|e| e.replacement == "if"));
    }

    #[test]
    fn test_mixed_case() {
        let source = "<?php\nIf ($a) { Return TRUE; }\n";
        let edits = check(source);

        assert!(edits.iter().any(|e| e.replacement == "if"));
        assert!(edits.iter().any(|e| e.replacement == "return"));
        assert!(edits.iter().any(|e| e.replacement == "true"));
    }

    #[test]
    fn test_class_keyword() {
        let source = "<?php\nCLASS Foo { }\n";
        let edits = check(source);

        assert!(edits.iter().any(|e| e.replacement == "class"));
    }

    #[test]
    fn test_function_keyword() {
        let source = "<?php\nFUNCTION test() { }\n";
        let edits = check(source);

        assert!(edits.iter().any(|e| e.replacement == "function"));
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'IF this is a STRING';\n";
        let edits = check(source);

        // Should not change keywords inside strings
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_comment() {
        let source = "<?php\n// IF this is a COMMENT\n$a = 1;\n";
        let edits = check(source);

        assert!(edits.is_empty());
    }

    #[test]
    fn test_type_keywords() {
        let source = "<?php\nfunction test(): INT { return 1; }\n";
        let edits = check(source);

        assert!(edits.iter().any(|e| e.replacement == "int"));
    }

    #[test]
    fn test_is_in_string() {
        assert!(is_in_string_or_comment("$a = '"));
        assert!(is_in_string_or_comment("$a = \""));
        assert!(!is_in_string_or_comment("$a = 'test'; "));
        assert!(is_in_string_or_comment("// comment "));
        assert!(is_in_string_or_comment("/* block "));
        assert!(!is_in_string_or_comment("/* block */ "));
    }
}
