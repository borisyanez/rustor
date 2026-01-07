//! Rule: Convert is_null($x) to $x === null
//!
//! This transformation improves performance by avoiding function call overhead.
//! Also handles negation: !is_null($x) → $x !== null

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for is_null calls that can be simplified
pub fn check_is_null<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = IsNullVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct IsNullVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for IsNullVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Handle !is_null($x) → $x !== null
        if let Expression::UnaryPrefix(unary) = expr {
            if let UnaryPrefixOperator::Not(_) = &unary.operator {
                if let Some(replacement) = try_transform_is_null(&unary.operand, self.source, true)
                {
                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Replace !is_null() with !== null for better performance",
                    ));
                    return false; // Don't traverse children, we handled this
                }
            }
        }

        // Handle is_null($x) → $x === null
        if let Some(replacement) = try_transform_is_null(expr, self.source, false) {
            self.edits.push(Edit::new(
                expr.span(),
                replacement,
                "Replace is_null() with === null for better performance",
            ));
            return false; // Don't traverse children
        }

        true // Continue traversal
    }
}

/// Try to transform an is_null() call, returning the replacement string if successful
fn try_transform_is_null(expr: &Expression<'_>, source: &str, negated: bool) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if name.eq_ignore_ascii_case("is_null") {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

                // is_null takes exactly 1 argument
                if args.len() == 1 {
                    // Skip if argument contains an assignment (would cause precedence issues)
                    // e.g., is_null($x = foo()) → $x = foo() === null has wrong precedence
                    if matches!(args[0].value(), Expression::Assignment(_)) {
                        return None;
                    }

                    let arg_span = args[0].span();
                    let arg_code =
                        &source[arg_span.start.offset as usize..arg_span.end.offset as usize];

                    let operator = if negated { "!==" } else { "===" };
                    return Some(format!("{} {} null", arg_code, operator));
                }
            }
        }
    }
    None
}

use crate::registry::{Category, Rule};

pub struct IsNullRule;

impl Rule for IsNullRule {
    fn name(&self) -> &'static str {
        "is_null"
    }

    fn description(&self) -> &'static str {
        "Convert is_null($x) to $x === null"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_is_null(program, source)
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
        check_is_null(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_is_null() {
        let source = "<?php is_null($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x === null;");
    }

    #[test]
    fn test_is_null_in_condition() {
        let source = "<?php if (is_null($x)) {}";
        assert_eq!(transform(source), "<?php if ($x === null) {}");
    }

    #[test]
    fn test_is_null_in_assignment() {
        let source = "<?php $result = is_null($x);";
        assert_eq!(transform(source), "<?php $result = $x === null;");
    }

    #[test]
    fn test_is_null_with_array_access() {
        let source = "<?php is_null($arr['key']);";
        assert_eq!(transform(source), "<?php $arr['key'] === null;");
    }

    #[test]
    fn test_is_null_with_method_call() {
        let source = "<?php is_null($obj->getValue());";
        assert_eq!(transform(source), "<?php $obj->getValue() === null;");
    }

    // ==================== Negation Tests ====================

    #[test]
    fn test_negated_is_null() {
        let source = "<?php !is_null($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x !== null;");
    }

    #[test]
    fn test_negated_is_null_in_condition() {
        let source = "<?php if (!is_null($x)) {}";
        assert_eq!(transform(source), "<?php if ($x !== null) {}");
    }

    #[test]
    fn test_negated_is_null_in_assignment() {
        let source = "<?php $result = !is_null($x);";
        assert_eq!(transform(source), "<?php $result = $x !== null;");
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_is_null() {
        let source = "<?php is_null($a); is_null($b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a === null; $b === null;");
    }

    #[test]
    fn test_mixed_is_null_and_negated() {
        let source = "<?php if (is_null($a) || !is_null($b)) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(
            transform(source),
            "<?php if ($a === null || $b !== null) {}"
        );
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_is_null_in_ternary() {
        let source = "<?php $result = is_null($x) ? 'yes' : 'no';";
        assert_eq!(transform(source), "<?php $result = $x === null ? 'yes' : 'no';");
    }

    #[test]
    fn test_is_null_in_binary_expression() {
        let source = "<?php if (is_null($x) && $y > 0) {}";
        assert_eq!(transform(source), "<?php if ($x === null && $y > 0) {}");
    }

    #[test]
    fn test_is_null_in_return() {
        let source = "<?php return is_null($x);";
        assert_eq!(transform(source), "<?php return $x === null;");
    }

    #[test]
    fn test_is_null_in_echo() {
        let source = "<?php echo is_null($x) ? 'null' : 'not null';";
        assert_eq!(transform(source), "<?php echo $x === null ? 'null' : 'not null';");
    }

    // ==================== Statement Context Tests ====================

    #[test]
    fn test_is_null_in_while_condition() {
        // Contains assignment, so we skip it (would cause precedence issues)
        let source = "<?php while (!is_null($item = next($arr))) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_is_null_in_for_condition() {
        let source = "<?php for ($i = 0; !is_null($arr[$i]); $i++) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_is_null_in_switch() {
        let source = r#"<?php
switch ($type) {
    case 'foo':
        if (is_null($x)) { break; }
        break;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_is_null_in_class_method() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return is_null($this->value);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_is_null_in_closure() {
        let source = r#"<?php
$fn = function($x) {
    return is_null($x);
};
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_is_null_in_arrow_function() {
        let source = r#"<?php
$fn = fn($x) => is_null($x);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_is_null_in_nested_closure() {
        let source = r#"<?php
$fn = function() {
    return function($x) {
        if (is_null($x)) {
            return true;
        }
        return false;
    };
};
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_is_null() {
        let source = "<?php IS_NULL($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_case_is_null() {
        let source = "<?php Is_Null($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_wrong_arg_count() {
        let source = "<?php is_null();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_is_null($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_assignment_inside_is_null() {
        // Skip: is_null($x = foo()) would become $x = foo() === null
        // which has wrong precedence (assigns boolean to $x)
        let source = "<?php if (is_null($result = getValue())) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_negated_assignment_inside_is_null() {
        // Same issue with negation
        let source = "<?php if (!is_null($result = getValue())) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_is_null_in_array() {
        let source = "<?php $arr = [is_null($a), !is_null($b)];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_is_null_in_short_ternary() {
        let source = "<?php $result = is_null($x) ?: 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_deeply_nested_is_null() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($condition) {
            foreach ($items as $item) {
                if (!is_null($item->value)) {
                    return true;
                }
            }
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
