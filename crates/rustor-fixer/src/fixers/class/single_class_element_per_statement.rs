//! Split multiple property/constant declarations into separate statements

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Splits multiple property declarations into separate statements
pub struct SingleClassElementPerStatementFixer;

impl Fixer for SingleClassElementPerStatementFixer {
    fn name(&self) -> &'static str {
        "single_class_element_per_statement"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "single_class_element_per_statement"
    }

    fn description(&self) -> &'static str {
        "Split multiple property declarations into separate statements"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Match property declarations with multiple variables
        // public $a, $b; -> public $a; public $b;
        // private int $a, $b; -> private int $a; private int $b;
        let prop_re = Regex::new(
            r"(?m)^([ \t]*)((?:public|protected|private|static|readonly|\s)+)(?:(\??\w+)\s+)?(\$\w+(?:\s*=\s*[^,;]+)?(?:\s*,\s*\$\w+(?:\s*=\s*[^,;]+)?)+)\s*;"
        ).unwrap();

        for cap in prop_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let indent = cap.get(1).unwrap().as_str();
            let modifiers = cap.get(2).unwrap().as_str().trim();
            let type_hint = cap.get(3).map(|m| m.as_str()).unwrap_or("");
            let vars = cap.get(4).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Split the variables
            let var_list: Vec<&str> = vars
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if var_list.len() <= 1 {
                continue;
            }

            // Generate individual declarations
            let mut statements = Vec::new();
            for var in var_list {
                if type_hint.is_empty() {
                    statements.push(format!("{}{} {};", indent, modifiers, var));
                } else {
                    statements.push(format!("{}{} {} {};", indent, modifiers, type_hint, var));
                }
            }

            let replacement = statements.join(line_ending);

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                "Split into separate property declarations".to_string(),
                "single_class_element_per_statement",
            ));
        }

        // Match constant declarations with multiple constants
        // const A = 1, B = 2; -> const A = 1; const B = 2;
        let const_re = Regex::new(
            r"(?m)^([ \t]*)((?:public|protected|private|final|\s)*const\s+)([A-Z_][A-Z0-9_]*\s*=\s*[^,;]+(?:\s*,\s*[A-Z_][A-Z0-9_]*\s*=\s*[^,;]+)+)\s*;"
        ).unwrap();

        for cap in const_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let indent = cap.get(1).unwrap().as_str();
            let const_prefix = cap.get(2).unwrap().as_str();
            let consts = cap.get(3).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Split the constants (need to be careful with = and ,)
            let const_list: Vec<&str> = split_const_declarations(consts);

            if const_list.len() <= 1 {
                continue;
            }

            // Generate individual declarations
            let statements: Vec<String> = const_list
                .iter()
                .map(|c| format!("{}{}{};", indent, const_prefix, c.trim()))
                .collect();

            let replacement = statements.join(line_ending);

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                "Split into separate constant declarations".to_string(),
                "single_class_element_per_statement",
            ));
        }

        edits
    }
}

fn split_const_declarations(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            ',' if depth == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < s.len() {
        result.push(&s[start..]);
    }

    result
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
        SingleClassElementPerStatementFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        })
    }

    #[test]
    fn test_single_property_unchanged() {
        let source = "<?php\nclass A {\n    public $a;\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_properties() {
        let source = "<?php\nclass A {\n    public $a, $b;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("public $a;"));
        assert!(edits[0].replacement.contains("public $b;"));
    }

    #[test]
    fn test_properties_with_values() {
        let source = "<?php\nclass A {\n    public $a = 1, $b = 2;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("public $a = 1;"));
        assert!(edits[0].replacement.contains("public $b = 2;"));
    }

    #[test]
    fn test_typed_properties() {
        let source = "<?php\nclass A {\n    public int $a, $b;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("public int $a;"));
        assert!(edits[0].replacement.contains("public int $b;"));
    }

    #[test]
    fn test_private_properties() {
        let source = "<?php\nclass A {\n    private $x, $y, $z;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("private $x;"));
        assert!(edits[0].replacement.contains("private $y;"));
        assert!(edits[0].replacement.contains("private $z;"));
    }

    #[test]
    fn test_constants() {
        let source = "<?php\nclass A {\n    const A = 1, B = 2;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("const A = 1;"));
        assert!(edits[0].replacement.contains("const B = 2;"));
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'public $a, $b;';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
