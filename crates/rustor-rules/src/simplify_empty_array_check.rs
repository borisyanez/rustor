//! Rule: Simplify is_array && empty check to === []
//!
//! When checking if a variable is an array and empty, use direct comparison
//! with an empty array instead.
//!
//! Transformation:
//! - `is_array($x) && empty($x)` → `$x === []`
//! - `empty($x) && is_array($x)` → `$x === []`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for is_array && empty patterns
pub fn check_simplify_empty_array_check<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyEmptyArrayCheckVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyEmptyArrayCheckVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SimplifyEmptyArrayCheckVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            if let Some(replacement) = try_transform_empty_array_check(binary, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Simplify is_array && empty to === []",
                ));
                return false;
            }
        }
        true
    }
}

/// Extract variable from is_array($var) call
fn extract_is_array_var<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if name.eq_ignore_ascii_case("is_array") {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
                if args.len() == 1 {
                    let arg_span = args[0].span();
                    return Some(
                        source[arg_span.start.offset as usize..arg_span.end.offset as usize]
                            .to_string(),
                    );
                }
            }
        }
    }
    None
}

/// Extract variable from empty($var) construct
fn extract_empty_var<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Construct(Construct::Empty(empty_construct)) = expr {
        let arg_span = empty_construct.value.span();
        return Some(
            source[arg_span.start.offset as usize..arg_span.end.offset as usize].to_string(),
        );
    }
    None
}

/// Try to transform is_array && empty pattern
fn try_transform_empty_array_check(binary: &Binary<'_>, source: &str) -> Option<String> {
    // Must be && operator
    if !matches!(binary.operator, BinaryOperator::And(_) | BinaryOperator::LowAnd(_)) {
        return None;
    }

    // Try both orders:
    // 1. is_array($x) && empty($x)
    // 2. empty($x) && is_array($x)

    let (is_array_var, empty_var) =
        if let Some(arr_var) = extract_is_array_var(binary.lhs, source) {
            let emp_var = extract_empty_var(binary.rhs, source)?;
            (arr_var, emp_var)
        } else if let Some(emp_var) = extract_empty_var(binary.lhs, source) {
            let arr_var = extract_is_array_var(binary.rhs, source)?;
            (arr_var, emp_var)
        } else {
            return None;
        };

    // Both must reference the same variable
    if is_array_var != empty_var {
        return None;
    }

    Some(format!("{} === []", is_array_var))
}

use crate::registry::{Category, Rule};

pub struct SimplifyEmptyArrayCheckRule;

impl Rule for SimplifyEmptyArrayCheckRule {
    fn name(&self) -> &'static str {
        "simplify_empty_array_check"
    }

    fn description(&self) -> &'static str {
        "Simplify is_array && empty to === []"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_empty_array_check(program, source)
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
        check_simplify_empty_array_check(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_is_array_first() {
        let source = "<?php is_array($x) && empty($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x === [];");
    }

    #[test]
    fn test_empty_first() {
        let source = "<?php empty($x) && is_array($x);";
        assert_eq!(transform(source), "<?php $x === [];");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_condition() {
        let source = "<?php if (is_array($arr) && empty($arr)) {}";
        assert_eq!(transform(source), "<?php if ($arr === []) {}");
    }

    #[test]
    fn test_in_assignment() {
        let source = "<?php $isEmpty = is_array($data) && empty($data);";
        assert_eq!(transform(source), "<?php $isEmpty = $data === [];");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return is_array($items) && empty($items);";
        assert_eq!(transform(source), "<?php return $items === [];");
    }

    // ==================== Case Insensitive ====================

    #[test]
    fn test_uppercase() {
        let source = "<?php IS_ARRAY($x) && EMPTY($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
is_array($a) && empty($a);
is_array($b) && empty($b);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_different_vars() {
        let source = "<?php is_array($x) && empty($y);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_or_operator() {
        let source = "<?php is_array($x) || empty($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_only_is_array() {
        let source = "<?php is_array($x) && $y;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_only_empty() {
        let source = "<?php empty($x) && $y;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_isset() {
        let source = "<?php is_array($x) && isset($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
