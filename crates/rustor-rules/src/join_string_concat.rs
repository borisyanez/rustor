//! Rule: Join adjacent string literals in concatenation
//!
//! When two string literals are concatenated, they can be combined into one.
//!
//! Transformation:
//! - `'Hi' . ' Tom'` → `'Hi Tom'`
//! - `"Hello" . " World"` → `"Hello World"`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for adjacent string literal concatenation
pub fn check_join_string_concat<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = JoinStringConcatVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct JoinStringConcatVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for JoinStringConcatVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            if matches!(binary.operator, BinaryOperator::StringConcat(_)) {
                if let Some(replacement) = try_join_strings(binary, self.source) {
                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Join adjacent string literals",
                    ));
                    return false;
                }
            }
        }
        true
    }
}

/// Get string value from a string literal expression
fn get_string_value<'a>(expr: &Expression<'a>, source: &str) -> Option<(String, char)> {
    if let Expression::Literal(Literal::String(string_lit)) = expr {
        let span = string_lit.span();
        let raw = &source[span.start.offset as usize..span.end.offset as usize];

        // Determine quote type and extract content
        if raw.starts_with('\'') && raw.ends_with('\'') {
            // Single quoted string - content is between quotes
            let content = &raw[1..raw.len()-1];
            return Some((content.to_string(), '\''));
        } else if raw.starts_with('"') && raw.ends_with('"') {
            // Double quoted string
            let content = &raw[1..raw.len()-1];
            return Some((content.to_string(), '"'));
        }
    }
    None
}

/// Try to join two string literals, returning the combined string if possible
fn try_join_strings(binary: &Binary<'_>, source: &str) -> Option<String> {
    let (left_value, left_quote) = get_string_value(binary.lhs, source)?;
    let (right_value, right_quote) = get_string_value(binary.rhs, source)?;

    // Don't join if strings contain newlines
    if left_value.contains('\n') || right_value.contains('\n') {
        return None;
    }

    // Use the quote type from the left string (or single quotes for consistency)
    let quote = if left_quote == right_quote { left_quote } else { '\'' };

    // Check if combined length is reasonable (avoid very long strings)
    let combined = format!("{}{}", left_value, right_value);
    if combined.len() >= 100 {
        return None;
    }

    Some(format!("{}{}{}", quote, combined, quote))
}

use crate::registry::{Category, Rule};

pub struct JoinStringConcatRule;

impl Rule for JoinStringConcatRule {
    fn name(&self) -> &'static str {
        "join_string_concat"
    }

    fn description(&self) -> &'static str {
        "Join adjacent string literals in concatenation"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_join_string_concat(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_join_string_concat(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_single_quotes() {
        let source = "<?php 'Hi' . ' Tom';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php 'Hi Tom';");
    }

    #[test]
    fn test_double_quotes() {
        let source = r#"<?php "Hello" . " World";"#;
        assert_eq!(transform(source), r#"<?php "Hello World";"#);
    }

    #[test]
    fn test_empty_strings() {
        let source = "<?php '' . 'hello';";
        assert_eq!(transform(source), "<?php 'hello';");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $name = 'Hello' . ' World';";
        assert_eq!(transform(source), "<?php $name = 'Hello World';");
    }

    #[test]
    fn test_in_echo() {
        let source = "<?php echo 'Hello' . ' there';";
        assert_eq!(transform(source), "<?php echo 'Hello there';");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return 'Result: ' . 'OK';";
        assert_eq!(transform(source), "<?php return 'Result: OK';");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = 'Hi' . ' there';
$b = 'Hello' . ' World';
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_variable_concat() {
        let source = "<?php 'Hi ' . $name;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_mixed_concat() {
        let source = "<?php $prefix . 'suffix';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_function_call() {
        let source = "<?php getName() . ' World';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
