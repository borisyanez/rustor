//! Class definition spacing and formatting

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures proper spacing in class definitions
pub struct ClassDefinitionFixer;

impl Fixer for ClassDefinitionFixer {
    fn name(&self) -> &'static str {
        "class_definition"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "class_definition"
    }

    fn description(&self) -> &'static str {
        "Fix spacing in class definition"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Fix multiple spaces after class/interface/trait keyword
        let keyword_re = Regex::new(r"\b(class|interface|trait)[ \t]{2,}(\w)").unwrap();

        for cap in keyword_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let keyword = cap.get(1).unwrap().as_str();
            let first_char = cap.get(2).unwrap().as_str();

            // Check not in string
            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{} {}", keyword, first_char),
                "Single space after class keyword".to_string(),
                "class_definition",
            ));
        }

        // Fix multiple spaces before extends
        let extends_re = Regex::new(r"(\w)[ \t]{2,}extends\b").unwrap();

        for cap in extends_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let last_char = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{} extends", last_char),
                "Single space before extends".to_string(),
                "class_definition",
            ));
        }

        // Fix multiple spaces after extends
        let after_extends_re = Regex::new(r"\bextends[ \t]{2,}(\w)").unwrap();

        for cap in after_extends_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let first_char = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("extends {}", first_char),
                "Single space after extends".to_string(),
                "class_definition",
            ));
        }

        // Fix multiple spaces before implements
        let implements_re = Regex::new(r"(\w)[ \t]{2,}implements\b").unwrap();

        for cap in implements_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let last_char = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{} implements", last_char),
                "Single space before implements".to_string(),
                "class_definition",
            ));
        }

        // Fix multiple spaces after implements
        let after_implements_re = Regex::new(r"\bimplements[ \t]{2,}(\w)").unwrap();

        for cap in after_implements_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let first_char = cap.get(1).unwrap().as_str();

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("implements {}", first_char),
                "Single space after implements".to_string(),
                "class_definition",
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
        ClassDefinitionFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nclass Foo extends Bar implements Baz {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_spaces_after_class() {
        let source = "<?php\nclass  Foo {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("class F"));
    }

    #[test]
    fn test_multiple_spaces_before_extends() {
        let source = "<?php\nclass Foo  extends Bar {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_spaces_after_extends() {
        let source = "<?php\nclass Foo extends  Bar {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_spaces_before_implements() {
        let source = "<?php\nclass Foo  implements Bar {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_spaces_after_implements() {
        let source = "<?php\nclass Foo implements  Bar {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_interface() {
        let source = "<?php\ninterface  Foo {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_trait() {
        let source = "<?php\ntrait  Foo {}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
    }
}
