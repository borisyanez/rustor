//! Rule: Convert pow() to ** operator
//!
//! The pow() function can be replaced with the ** exponentiation operator
//! which was introduced in PHP 5.6. The operator form is more concise
//! and often more readable.
//!
//! Example: `pow($x, 2)` → `$x ** 2`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for pow() calls that can be replaced with **
pub fn check_pow_to_operator<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = PowToOperatorVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct PowToOperatorVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for PowToOperatorVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_pow(expr, self.source) {
            self.edits.push(edit);
            return false; // Don't traverse children to avoid overlapping edits
        }
        true // Continue traversal
    }
}

/// Try to transform a pow() call to ** operator
fn try_transform_pow(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if name.eq_ignore_ascii_case("pow") {
                let args = &func_call.argument_list.arguments;

                // Only transform pow() with exactly 2 arguments
                if args.len() != 2 {
                    return None;
                }

                // Get the base and exponent expressions
                let base_expr = args.first()?.value();
                let exp_expr = args.get(1)?.value();

                let base_span = base_expr.span();
                let exp_span = exp_expr.span();

                let base_str =
                    &source[base_span.start.offset as usize..base_span.end.offset as usize];
                let exp_str =
                    &source[exp_span.start.offset as usize..exp_span.end.offset as usize];

                // Check if base needs parentheses (for complex expressions)
                let base_needs_parens = needs_parentheses(base_expr);
                let exp_needs_parens = needs_parentheses(exp_expr);

                let replacement = format!(
                    "{}{}{}",
                    if base_needs_parens {
                        format!("({})", base_str)
                    } else {
                        base_str.to_string()
                    },
                    " ** ",
                    if exp_needs_parens {
                        format!("({})", exp_str)
                    } else {
                        exp_str.to_string()
                    }
                );

                let call_span = expr.span();
                return Some(Edit::new(
                    call_span,
                    replacement,
                    "Replace pow() with ** operator",
                ));
            }
        }
    }
    None
}

/// Check if an expression needs parentheses when used with ** operator
fn needs_parentheses(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::Binary(_)
            | Expression::Conditional(_)
            | Expression::Assignment(_)
            | Expression::UnaryPrefix(_)
    )
}

use crate::registry::Rule;

pub struct PowToOperatorRule;

impl Rule for PowToOperatorRule {
    fn name(&self) -> &'static str {
        "pow_to_operator"
    }

    fn description(&self) -> &'static str {
        "Convert pow($x, $n) to $x ** $n"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_pow_to_operator(program, source)
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
        check_pow_to_operator(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_pow() {
        let source = "<?php pow($x, 2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x ** 2;");
    }

    #[test]
    fn test_pow_with_variables() {
        let source = "<?php pow($base, $exp);";
        assert_eq!(transform(source), "<?php $base ** $exp;");
    }

    #[test]
    fn test_pow_with_numbers() {
        let source = "<?php pow(2, 10);";
        assert_eq!(transform(source), "<?php 2 ** 10;");
    }

    #[test]
    fn test_pow_in_assignment() {
        let source = "<?php $result = pow($x, 3);";
        assert_eq!(transform(source), "<?php $result = $x ** 3;");
    }

    #[test]
    fn test_pow_in_return() {
        let source = "<?php return pow($n, 2);";
        assert_eq!(transform(source), "<?php return $n ** 2;");
    }

    #[test]
    fn test_pow_in_echo() {
        let source = "<?php echo pow(2, 8);";
        assert_eq!(transform(source), "<?php echo 2 ** 8;");
    }

    // ==================== Complex Expression Tests ====================

    #[test]
    fn test_pow_with_binary_base() {
        // Binary expressions need parentheses
        let source = "<?php pow($a + $b, 2);";
        assert_eq!(transform(source), "<?php ($a + $b) ** 2;");
    }

    #[test]
    fn test_pow_with_binary_exponent() {
        let source = "<?php pow($x, $n - 1);";
        assert_eq!(transform(source), "<?php $x ** ($n - 1);");
    }

    #[test]
    fn test_pow_with_function_call_base() {
        let source = "<?php pow(abs($x), 2);";
        assert_eq!(transform(source), "<?php abs($x) ** 2;");
    }

    #[test]
    fn test_pow_with_array_access() {
        let source = "<?php pow($arr[0], $arr[1]);";
        assert_eq!(transform(source), "<?php $arr[0] ** $arr[1];");
    }

    #[test]
    fn test_pow_with_property_access() {
        let source = "<?php pow($obj->value, 2);";
        assert_eq!(transform(source), "<?php $obj->value ** 2;");
    }

    #[test]
    fn test_pow_with_negative_exponent() {
        let source = "<?php pow($x, -1);";
        assert_eq!(transform(source), "<?php $x ** (-1);");
    }

    #[test]
    fn test_pow_with_float_exponent() {
        let source = "<?php pow($x, 0.5);";
        assert_eq!(transform(source), "<?php $x ** 0.5;");
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_pow() {
        let source = "<?php pow($a, 2); pow($b, 3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a ** 2; $b ** 3;");
    }

    #[test]
    fn test_nested_pow() {
        // Only outermost pow is transformed to avoid overlapping edits
        // Run tool again to transform inner pow
        let source = "<?php pow(pow($x, 2), 3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php pow($x, 2) ** 3;");
    }

    #[test]
    fn test_pow_in_expression() {
        let source = "<?php $y = pow($x, 2) + pow($x, 3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $y = $x ** 2 + $x ** 3;");
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_pow_in_function() {
        let source = r#"<?php
function square($n) {
    return pow($n, 2);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_pow_in_class_method() {
        let source = r#"<?php
class Math {
    public function power($base, $exp) {
        return pow($base, $exp);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_pow_in_ternary() {
        let source = "<?php $result = $positive ? pow($x, 2) : pow($x, 3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_pow_in_array() {
        let source = "<?php $squares = [pow(1, 2), pow(2, 2), pow(3, 2)];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 3);
    }

    #[test]
    fn test_pow_as_function_arg() {
        let source = "<?php doSomething(pow($x, 2));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_deeply_nested() {
        let source = r#"<?php
class Calculator {
    public function compute($x) {
        if ($x > 0) {
            foreach (range(1, 10) as $n) {
                $result = pow($x, $n);
            }
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_pow() {
        let source = "<?php POW($x, 2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x ** 2;");
    }

    #[test]
    fn test_mixed_case_pow() {
        let source = "<?php Pow($x, 2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_single_arg() {
        // pow() requires exactly 2 arguments
        let source = "<?php pow($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_three_args() {
        // pow() with 3 args doesn't exist, but we should skip it
        let source = "<?php pow($x, 2, 3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $math->pow($x, 2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_static_method() {
        let source = "<?php Math::pow($x, 2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_pow($x, 2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_bcpow() {
        // bcpow is a different function for arbitrary precision
        let source = "<?php bcpow($x, 2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
