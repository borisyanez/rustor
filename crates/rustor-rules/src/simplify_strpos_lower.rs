//! Rule: Simplify strpos(strtolower($x), 'needle') to stripos($x, 'needle')
//!
//! When searching case-insensitively, use stripos() directly instead of
//! converting to lowercase first.
//!
//! Transformation:
//! - `strpos(strtolower($x), 'needle')` → `stripos($x, 'needle')`
//!
//! Note: Only applies when the needle is lowercase (no uppercase letters).

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for strpos(strtolower()) patterns
pub fn check_simplify_strpos_lower<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyStrposLowerVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyStrposLowerVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SimplifyStrposLowerVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(replacement) = try_transform_strpos_lower(func_call, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Simplify strpos(strtolower()) to stripos()",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to transform strpos(strtolower()), returning the replacement if successful
fn try_transform_strpos_lower(func_call: &FunctionCall<'_>, source: &str) -> Option<String> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !name.eq_ignore_ascii_case("strpos") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // strpos needs at least 2 arguments
    if args.len() < 2 {
        return None;
    }

    // First argument must be a strtolower() call
    let first_arg_value = args[0].value();
    let inner_arg = if let Expression::Call(Call::Function(inner_call)) = first_arg_value {
        if let Expression::Identifier(ident) = inner_call.function {
            let inner_name_span = ident.span();
            let inner_name = &source[inner_name_span.start.offset as usize..inner_name_span.end.offset as usize];

            if !inner_name.eq_ignore_ascii_case("strtolower") {
                return None;
            }

            // Get the argument to strtolower
            let inner_args: Vec<_> = inner_call.argument_list.arguments.iter().collect();
            if inner_args.len() != 1 {
                return None;
            }

            let inner_arg_span = inner_args[0].span();
            &source[inner_arg_span.start.offset as usize..inner_arg_span.end.offset as usize]
        } else {
            return None;
        }
    } else {
        return None;
    };

    // Second argument (needle) - check if it's a lowercase string
    let second_arg_span = args[1].span();
    let needle_text = &source[second_arg_span.start.offset as usize..second_arg_span.end.offset as usize];

    // Check if needle is a string literal with no uppercase letters
    let second_arg_value = args[1].value();
    if let Expression::Literal(Literal::String(string_lit)) = second_arg_value {
        let string_span = string_lit.span();
        let string_content = &source[string_span.start.offset as usize..string_span.end.offset as usize];

        // Skip if string contains uppercase letters (between quotes)
        // Remove quotes and check content
        let content = string_content.trim_matches(|c| c == '\'' || c == '"');
        if content.chars().any(|c| c.is_ascii_uppercase()) {
            return None;
        }
    }

    // Build replacement: stripos($inner_arg, $needle, ...)
    let mut replacement = format!("stripos({}, {}", inner_arg, needle_text);

    // Add optional offset argument if present
    if args.len() > 2 {
        let offset_span = args[2].span();
        let offset_text = &source[offset_span.start.offset as usize..offset_span.end.offset as usize];
        replacement.push_str(", ");
        replacement.push_str(offset_text);
    }

    replacement.push(')');
    Some(replacement)
}

use crate::registry::{Category, Rule};

pub struct SimplifyStrposLowerRule;

impl Rule for SimplifyStrposLowerRule {
    fn name(&self) -> &'static str {
        "simplify_strpos_lower"
    }

    fn description(&self) -> &'static str {
        "Simplify strpos(strtolower()) to stripos()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_strpos_lower(program, source)
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
        check_simplify_strpos_lower(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_basic() {
        let source = "<?php strpos(strtolower($var), 'needle');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php stripos($var, 'needle');");
    }

    #[test]
    fn test_in_condition() {
        let source = "<?php if (strpos(strtolower($str), 'test') !== false) {}";
        assert_eq!(
            transform(source),
            "<?php if (stripos($str, 'test') !== false) {}"
        );
    }

    #[test]
    fn test_double_quoted_needle() {
        let source = r#"<?php strpos(strtolower($x), "hello");"#;
        assert_eq!(transform(source), r#"<?php stripos($x, "hello");"#);
    }

    #[test]
    fn test_with_offset() {
        let source = "<?php strpos(strtolower($str), 'a', 5);";
        assert_eq!(transform(source), "<?php stripos($str, 'a', 5);");
    }

    #[test]
    fn test_with_expression() {
        let source = "<?php strpos(strtolower($obj->getText()), 'word');";
        assert_eq!(transform(source), "<?php stripos($obj->getText(), 'word');");
    }

    #[test]
    fn test_uppercase_function() {
        let source = "<?php STRPOS(STRTOLOWER($x), 'test');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple() {
        let source = r#"<?php
strpos(strtolower($a), 'x');
strpos(strtolower($b), 'y');
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_uppercase_needle() {
        // Don't transform if needle contains uppercase (wouldn't match after strtolower)
        let source = "<?php strpos(strtolower($var), 'Hello');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_strtolower() {
        let source = "<?php strpos($var, 'needle');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_strtoupper() {
        let source = "<?php strpos(strtoupper($var), 'needle');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->strpos(strtolower($x), 'test');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_single_arg() {
        let source = "<?php strpos(strtolower($var));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
