//! Ensure visibility is declared on methods and properties

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures visibility modifiers are declared on methods and properties
pub struct VisibilityRequiredFixer;

impl Fixer for VisibilityRequiredFixer {
    fn name(&self) -> &'static str {
        "visibility_required"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "visibility_required"
    }

    fn description(&self) -> &'static str {
        "Ensure visibility is declared on methods and properties"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Check for function declarations without visibility in class context
        // This is a simplified check - real implementation would need AST
        let func_re = Regex::new(r"(?m)^([ \t]*)(static\s+)?function\s+(\w+)\s*\(").unwrap();

        for cap in func_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let indent = cap.get(1).unwrap().as_str();
            let is_static = cap.get(2).is_some();
            let func_name = cap.get(3).unwrap().as_str();

            // Skip if not in a class context (check for class keyword before)
            if !is_in_class_context(&source[..full_match.start()]) {
                continue;
            }

            // Skip if already has visibility
            let before_line = get_line_before(source, full_match.start());
            if has_visibility(before_line) || before_line.contains("abstract") {
                continue;
            }

            // Skip constructors __construct and magic methods that might be special
            if func_name.starts_with("__") {
                // Still need visibility, but skip for now as detection is complex
            }

            let replacement = if is_static {
                format!("{}public static function {}(", indent, func_name)
            } else {
                format!("{}public function {}(", indent, func_name)
            };

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                replacement,
                format!("Add visibility modifier to method '{}'", func_name),
                "visibility_required",
            ));
        }

        // Check for property declarations without visibility
        // Match: var $prop or just $prop at class level
        let var_re = Regex::new(r"(?m)^([ \t]*)var\s+\$(\w+)").unwrap();

        for cap in var_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let indent = cap.get(1).unwrap().as_str();
            let prop_name = cap.get(2).unwrap().as_str();

            if !is_in_class_context(&source[..full_match.start()]) {
                continue;
            }

            edits.push(edit_with_rule(
                full_match.start(),
                full_match.end(),
                format!("{}public ${}", indent, prop_name),
                format!("Replace 'var' with 'public' for property '{}'", prop_name),
                "visibility_required",
            ));
        }

        edits
    }
}

fn is_in_class_context(before: &str) -> bool {
    // Simple heuristic: count class/interface/trait vs closing braces
    // A more robust implementation would use AST
    let class_pattern = Regex::new(r"\b(class|interface|trait)\s+\w+").unwrap();
    let class_count = class_pattern.find_iter(before).count();

    // Count opening and closing braces after last class declaration
    if let Some(last_class) = class_pattern.find_iter(before).last() {
        let after_class = &before[last_class.end()..];
        let opens = after_class.matches('{').count();
        let closes = after_class.matches('}').count();
        return opens > closes;
    }

    class_count > 0
}

fn get_line_before(source: &str, pos: usize) -> &str {
    let before = &source[..pos];
    // Find the start of the current line
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    &source[line_start..pos]
}

fn has_visibility(line: &str) -> bool {
    let vis_re = Regex::new(r"\b(public|protected|private)\b").unwrap();
    vis_re.is_match(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        VisibilityRequiredFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nclass Foo {\n    public function bar() {}\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_method_without_visibility() {
        let source = "<?php\nclass Foo {\n    function bar() {}\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("public function bar("));
    }

    #[test]
    fn test_static_method_without_visibility() {
        let source = "<?php\nclass Foo {\n    static function bar() {}\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("public static function bar("));
    }

    #[test]
    fn test_var_property() {
        let source = "<?php\nclass Foo {\n    var $bar;\n}\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("public $bar"));
    }

    #[test]
    fn test_skip_outside_class() {
        let source = "<?php\nfunction bar() {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_protected_unchanged() {
        let source = "<?php\nclass Foo {\n    protected function bar() {}\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_private_unchanged() {
        let source = "<?php\nclass Foo {\n    private function bar() {}\n}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }
}
