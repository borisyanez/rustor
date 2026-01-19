//! Rule: Simplify in_array(array_values()) to in_array()
//!
//! The array_values() call is unnecessary when used as the second argument
//! to in_array(), since in_array() only checks values, not keys.
//!
//! Transformation:
//! - `in_array($needle, array_values($arr))` → `in_array($needle, $arr)`
//! - `in_array($needle, array_values($arr), true)` → `in_array($needle, $arr, true)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for in_array(array_values()) patterns
pub fn check_simplify_in_array_values<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyInArrayValuesVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyInArrayValuesVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SimplifyInArrayValuesVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(replacement) = try_transform_in_array_values(func_call, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Remove unnecessary array_values() in in_array()",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to transform in_array(array_values()), returning the replacement if successful
fn try_transform_in_array_values(func_call: &FunctionCall<'_>, source: &str) -> Option<String> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !name.eq_ignore_ascii_case("in_array") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // in_array needs at least 2 arguments
    if args.len() < 2 {
        return None;
    }

    // Second argument must be array_values() call
    let second_arg_value = args[1].value();
    let inner_arg = if let Expression::Call(Call::Function(inner_call)) = second_arg_value {
        if let Expression::Identifier(ident) = inner_call.function {
            let inner_name_span = ident.span();
            let inner_name = &source[inner_name_span.start.offset as usize..inner_name_span.end.offset as usize];

            if !inner_name.eq_ignore_ascii_case("array_values") {
                return None;
            }

            // Get the argument to array_values
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

    // Get first argument (needle)
    let first_arg_span = args[0].span();
    let needle = &source[first_arg_span.start.offset as usize..first_arg_span.end.offset as usize];

    // Build replacement
    let mut replacement = format!("in_array({}, {}", needle, inner_arg);

    // Add optional third argument (strict) if present
    if args.len() > 2 {
        let third_arg_span = args[2].span();
        let strict = &source[third_arg_span.start.offset as usize..third_arg_span.end.offset as usize];
        replacement.push_str(", ");
        replacement.push_str(strict);
    }

    replacement.push(')');
    Some(replacement)
}

use crate::registry::{Category, Rule};

pub struct SimplifyInArrayValuesRule;

impl Rule for SimplifyInArrayValuesRule {
    fn name(&self) -> &'static str {
        "simplify_in_array_values"
    }

    fn description(&self) -> &'static str {
        "Remove unnecessary array_values() in in_array()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_in_array_values(program, source)
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
        check_simplify_in_array_values(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php in_array($key, array_values($arr));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php in_array($key, $arr);");
    }

    #[test]
    fn test_with_strict() {
        let source = "<?php in_array($key, array_values($arr), true);";
        assert_eq!(transform(source), "<?php in_array($key, $arr, true);");
    }

    #[test]
    fn test_with_strict_false() {
        let source = "<?php in_array($key, array_values($arr), false);";
        assert_eq!(transform(source), "<?php in_array($key, $arr, false);");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_condition() {
        let source = "<?php if (in_array('x', array_values($data))) {}";
        assert_eq!(transform(source), "<?php if (in_array('x', $data)) {}");
    }

    #[test]
    fn test_in_assignment() {
        let source = "<?php $found = in_array($needle, array_values($haystack));";
        assert_eq!(transform(source), "<?php $found = in_array($needle, $haystack);");
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn test_string_needle() {
        let source = "<?php in_array('value', array_values($arr));";
        assert_eq!(transform(source), "<?php in_array('value', $arr);");
    }

    #[test]
    fn test_method_call_array() {
        let source = "<?php in_array($key, array_values($obj->getData()));";
        assert_eq!(transform(source), "<?php in_array($key, $obj->getData());");
    }

    #[test]
    fn test_function_call_needle() {
        let source = "<?php in_array(getValue(), array_values($arr));";
        assert_eq!(transform(source), "<?php in_array(getValue(), $arr);");
    }

    // ==================== Case Insensitive ====================

    #[test]
    fn test_uppercase() {
        let source = "<?php IN_ARRAY($k, ARRAY_VALUES($arr));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
in_array($a, array_values($x));
in_array($b, array_values($y));
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_no_array_values() {
        let source = "<?php in_array($key, $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_array_keys() {
        let source = "<?php in_array($key, array_keys($arr));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_single_arg() {
        let source = "<?php in_array($key);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->in_array($key, array_values($arr));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
