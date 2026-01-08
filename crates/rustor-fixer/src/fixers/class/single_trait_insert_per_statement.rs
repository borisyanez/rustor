//! Split multiple trait imports into separate statements

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Splits multiple trait imports into separate statements
pub struct SingleTraitInsertPerStatementFixer;

impl Fixer for SingleTraitInsertPerStatementFixer {
    fn name(&self) -> &'static str {
        "single_trait_insert_per_statement"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "single_trait_insert_per_statement"
    }

    fn description(&self) -> &'static str {
        "Split multiple trait imports into separate statements"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let line_ending = config.line_ending.as_str();

        // Match trait use statements with multiple traits
        // use TraitA, TraitB; -> use TraitA; use TraitB;
        // Skip those with conflict resolution blocks { }
        let trait_re = Regex::new(
            r"(?m)^([ \t]*)(use\s+)([\w\\]+(?:\s*,\s*[\w\\]+)+)\s*;"
        ).unwrap();

        for cap in trait_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let indent = cap.get(1).unwrap().as_str();
            let use_prefix = cap.get(2).unwrap().as_str();
            let traits = cap.get(3).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Make sure this is inside a class (after class {)
            // by checking context - a simple heuristic
            let before = &source[..full_match.start()];
            if !before.contains("class ") && !before.contains("trait ") {
                // This might be a namespace use, skip it
                // Check if it looks like a namespace import
                if traits.contains("\\") && !before.trim_end().ends_with('{') {
                    continue;
                }
            }

            // Split the traits
            let trait_list: Vec<&str> = traits
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if trait_list.len() <= 1 {
                continue;
            }

            // Generate individual use statements
            let statements: Vec<String> = trait_list
                .iter()
                .map(|t| format!("{}{}{};", indent, use_prefix, t))
                .collect();

            let replacement = statements.join(line_ending);

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                "Split into separate trait imports".to_string(),
                "single_trait_insert_per_statement",
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
        SingleTraitInsertPerStatementFixer.check(source, &FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        })
    }

    #[test]
    fn test_single_trait_unchanged() {
        let source = "<?php\nclass A {\n    use TraitA;\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_traits() {
        let source = "<?php\nclass A {\n    use TraitA, TraitB;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use TraitA;"));
        assert!(edits[0].replacement.contains("use TraitB;"));
    }

    #[test]
    fn test_three_traits() {
        let source = "<?php\nclass A {\n    use TraitA, TraitB, TraitC;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use TraitA;"));
        assert!(edits[0].replacement.contains("use TraitB;"));
        assert!(edits[0].replacement.contains("use TraitC;"));
    }

    #[test]
    fn test_namespaced_traits() {
        let source = "<?php\nclass A {\n    use App\\TraitA, App\\TraitB;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("use App\\TraitA;"));
        assert!(edits[0].replacement.contains("use App\\TraitB;"));
    }

    #[test]
    fn test_preserves_indent() {
        let source = "<?php\nclass A {\n        use TraitA, TraitB;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        // Each line should have the same indent
        for line in edits[0].replacement.lines() {
            assert!(line.starts_with("        use "));
        }
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'use TraitA, TraitB;';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_in_trait() {
        let source = "<?php\ntrait A {\n    use TraitA, TraitB;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }
}
