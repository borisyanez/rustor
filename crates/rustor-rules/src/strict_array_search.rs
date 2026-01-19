//! Rule: Add strict comparison flag to array_search
//!
//! Makes array_search use strict comparison by adding `true` as the third argument.
//!
//! Transformation:
//! - `array_search($value, $arr)` → `array_search($value, $arr, true)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for array_search calls without strict flag
pub fn check_strict_array_search<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = StrictArraySearchVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StrictArraySearchVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for StrictArraySearchVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_add_strict_flag(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to add strict flag to array_search
fn try_add_strict_flag(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name is "array_search"
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("array_search") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Only transform if there are exactly 2 arguments
    if args.len() != 2 {
        return None;
    }

    // Get the full function call and add ", true" before the closing paren
    let func_span = func_call.span();
    let func_text = &source[func_span.start.offset as usize..func_span.end.offset as usize];

    // Insert ", true" before the closing parenthesis
    let new_text = format!("{}, true)", &func_text[..func_text.len() - 1]);

    Some(Edit::new(
        func_span,
        new_text,
        "Add strict flag to array_search",
    ))
}

use crate::registry::{Category, Rule};

pub struct StrictArraySearchRule;

impl Rule for StrictArraySearchRule {
    fn name(&self) -> &'static str {
        "strict_array_search"
    }

    fn description(&self) -> &'static str {
        "Add strict comparison flag to array_search"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_strict_array_search(program, source)
    }

    fn category(&self) -> Category {
        Category::Performance
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
        check_strict_array_search(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php array_search($value, $items);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php array_search($value, $items, true);");
    }

    #[test]
    fn test_with_string_needle() {
        let source = r#"<?php array_search("needle", $haystack);"#;
        assert_eq!(transform(source), r#"<?php array_search("needle", $haystack, true);"#);
    }

    #[test]
    fn test_with_int_needle() {
        let source = "<?php array_search(42, $arr);";
        assert_eq!(transform(source), "<?php array_search(42, $arr, true);");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $key = array_search($val, $arr);";
        assert_eq!(transform(source), "<?php $key = array_search($val, $arr, true);");
    }

    #[test]
    fn test_in_condition() {
        let source = "<?php if (array_search($x, $items) !== false) {}";
        assert_eq!(transform(source), "<?php if (array_search($x, $items, true) !== false) {}");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return array_search($needle, $haystack);";
        assert_eq!(transform(source), "<?php return array_search($needle, $haystack, true);");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = array_search($x, $items);
$b = array_search($y, $other);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_already_strict() {
        // Already has third argument
        let source = "<?php array_search($value, $items, true);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_with_false() {
        // Has explicit false - don't change
        let source = "<?php array_search($value, $items, false);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_one_arg() {
        // Only one argument - invalid call, don't transform
        let source = "<?php array_search($value);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php in_array($value, $items);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
