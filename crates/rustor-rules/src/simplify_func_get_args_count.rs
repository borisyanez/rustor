//! Rule: Simplify count(func_get_args()) to func_num_args()
//!
//! Instead of getting all arguments and counting them, use the dedicated
//! func_num_args() function which is more efficient.
//!
//! Transformation:
//! - `count(func_get_args())` → `func_num_args()`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for count(func_get_args()) patterns
pub fn check_simplify_func_get_args_count<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyFuncGetArgsCountVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyFuncGetArgsCountVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SimplifyFuncGetArgsCountVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(replacement) = try_transform_func_get_args_count(func_call, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Simplify count(func_get_args()) to func_num_args()",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to transform count(func_get_args()), returning the replacement if successful
fn try_transform_func_get_args_count(func_call: &FunctionCall<'_>, source: &str) -> Option<String> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !name.eq_ignore_ascii_case("count") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // count needs exactly 1 argument for this transformation
    if args.len() != 1 {
        return None;
    }

    // First argument must be func_get_args() call
    let first_arg_value = args[0].value();
    if let Expression::Call(Call::Function(inner_call)) = first_arg_value {
        if let Expression::Identifier(ident) = inner_call.function {
            let inner_name_span = ident.span();
            let inner_name = &source[inner_name_span.start.offset as usize..inner_name_span.end.offset as usize];

            if inner_name.eq_ignore_ascii_case("func_get_args") {
                // func_get_args() should have no arguments
                if inner_call.argument_list.arguments.is_empty() {
                    return Some("func_num_args()".to_string());
                }
            }
        }
    }

    None
}

use crate::registry::{Category, Rule};

pub struct SimplifyFuncGetArgsCountRule;

impl Rule for SimplifyFuncGetArgsCountRule {
    fn name(&self) -> &'static str {
        "simplify_func_get_args_count"
    }

    fn description(&self) -> &'static str {
        "Simplify count(func_get_args()) to func_num_args()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_func_get_args_count(program, source)
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
        check_simplify_func_get_args_count(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php count(func_get_args());";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php func_num_args();");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $num = count(func_get_args());";
        assert_eq!(transform(source), "<?php $num = func_num_args();");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return count(func_get_args());";
        assert_eq!(transform(source), "<?php return func_num_args();");
    }

    #[test]
    fn test_in_condition() {
        let source = "<?php if (count(func_get_args()) > 0) {}";
        assert_eq!(transform(source), "<?php if (func_num_args() > 0) {}");
    }

    #[test]
    fn test_in_function() {
        let source = r#"<?php
function test() {
    return count(func_get_args());
}
"#;
        assert!(transform(source).contains("func_num_args()"));
    }

    // ==================== Case Insensitive ====================

    #[test]
    fn test_uppercase() {
        let source = "<?php COUNT(FUNC_GET_ARGS());";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_case() {
        let source = "<?php Count(Func_Get_Args());";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = count(func_get_args());
$b = count(func_get_args());
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_count_other() {
        let source = "<?php count($arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_sizeof() {
        let source = "<?php sizeof(func_get_args());";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_count_with_mode() {
        // count() with COUNT_RECURSIVE should not be transformed
        let source = "<?php count(func_get_args(), COUNT_RECURSIVE);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->count(func_get_args());";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_func_get_arg() {
        // func_get_arg (singular) is different
        let source = "<?php count(func_get_arg(0));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
