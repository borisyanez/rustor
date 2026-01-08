//! Rule: single_in_array_to_compare
//!
//! Converts `in_array($needle, [$value], true)` to `$needle === $value`
//!
//! Patterns:
//! - `in_array($x, [$y], true)` → `$x === $y`
//! - `in_array($x, [$y])` → `$x == $y`
//! - `!in_array($x, [$y], true)` → `$x !== $y`
//! - `!in_array($x, [$y])` → `$x != $y`
//!
//! Why: Single-element array comparison is more readable as direct comparison.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub struct SingleInArrayToCompareRule;

impl Rule for SingleInArrayToCompareRule {
    fn name(&self) -> &'static str {
        "single_in_array_to_compare"
    }

    fn description(&self) -> &'static str {
        "Convert in_array() with single element to direct comparison"
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None // Works in all PHP versions
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        let mut visitor = SingleInArrayVisitor {
            source,
            edits: Vec::new(),
        };
        visitor.visit_program(program, source);
        visitor.edits
    }
}

struct SingleInArrayVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SingleInArrayVisitor<'s> {
    /// Get function name from FunctionCall
    fn get_func_name(&self, call: &FunctionCall<'_>) -> Option<String> {
        let span = call.function.span();
        Some(self.source[span.start.offset as usize..span.end.offset as usize].to_string())
    }

    /// Get source text for an argument
    fn get_arg_text(&self, arg: &Argument<'_>) -> String {
        let span = arg.span();
        self.source[span.start.offset as usize..span.end.offset as usize].to_string()
    }

    /// Check if the second argument is a single-element array literal
    fn get_single_array_value(&self, arg: &Argument<'_>) -> Option<String> {
        let expr = match arg {
            Argument::Positional(pos) => &pos.value,
            Argument::Named(named) => &named.value,
        };

        if let Expression::Array(array) = expr {
            // Check if array has exactly one element
            let elements: Vec<_> = array.elements.iter().collect();
            if elements.len() == 1 {
                if let ArrayElement::Value(val) = &elements[0] {
                    let span = val.value.span();
                    return Some(self.source[span.start.offset as usize..span.end.offset as usize].to_string());
                }
            }
        }

        None
    }

    /// Check if in_array has the strict flag (3rd argument is true)
    fn has_strict_flag(&self, call: &FunctionCall<'_>) -> bool {
        let args: Vec<_> = call.argument_list.arguments.iter().collect();
        if args.len() >= 3 {
            let text = self.get_arg_text(&args[2]).to_lowercase();
            text == "true"
        } else {
            false
        }
    }

    /// Process a potential in_array call, returning replacement text if matched
    fn process_in_array(&self, call: &FunctionCall<'_>, negated: bool) -> Option<String> {
        let name = self.get_func_name(call)?;
        if !name.eq_ignore_ascii_case("in_array") {
            return None;
        }

        let args: Vec<_> = call.argument_list.arguments.iter().collect();
        if args.len() < 2 {
            return None;
        }

        // Get the single array value from second argument
        let array_value = self.get_single_array_value(&args[1])?;

        // Get the needle (first argument)
        let needle = self.get_arg_text(&args[0]);

        // Determine the comparison operator
        let is_strict = self.has_strict_flag(call);
        let operator = match (negated, is_strict) {
            (false, true) => "===",
            (false, false) => "==",
            (true, true) => "!==",
            (true, false) => "!=",
        };

        Some(format!("{} {} {}", needle, operator, array_value))
    }
}

impl<'a, 's> Visitor<'a> for SingleInArrayVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Handle negated case: !in_array(...)
        if let Expression::UnaryPrefix(unary) = expr {
            if let UnaryPrefixOperator::Not(_) = &unary.operator {
                if let Expression::Call(Call::Function(call)) = &unary.operand {
                    if let Some(replacement) = self.process_in_array(call, true) {
                        self.edits.push(Edit::new(
                            expr.span(),
                            replacement,
                            "Use direct comparison instead of in_array() with single element",
                        ));
                        // Return false to skip visiting the inner in_array() call
                        return false;
                    }
                }
            }
        }

        // Handle non-negated case: in_array(...)
        if let Expression::Call(Call::Function(call)) = expr {
            if let Some(replacement) = self.process_in_array(call, false) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Use direct comparison instead of in_array() with single element",
                ));
                return true;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;

    fn parse_and_check(code: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, code);
        SingleInArrayToCompareRule.check(&program, code)
    }

    #[test]
    fn test_strict_comparison() {
        let code = r#"<?php
if (in_array($type, ['$this'], true)) {
    return true;
}
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$type === '$this'"));
    }

    #[test]
    fn test_loose_comparison() {
        let code = r#"<?php
$result = in_array($value, ['test']);
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$value == 'test'"));
    }

    #[test]
    fn test_negated_strict() {
        let code = r#"<?php
if (!in_array($x, [5], true)) {
    return false;
}
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$x !== 5"));
    }

    #[test]
    fn test_negated_loose() {
        let code = r#"<?php
$check = !in_array($name, ['admin']);
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$name != 'admin'"));
    }

    #[test]
    fn test_skip_multiple_elements() {
        let code = r#"<?php
in_array($x, ['a', 'b'], true);
"#;
        let edits = parse_and_check(code);
        assert!(edits.is_empty(), "Should not transform arrays with multiple elements");
    }

    #[test]
    fn test_skip_variable_array() {
        let code = r#"<?php
in_array($x, $array, true);
"#;
        let edits = parse_and_check(code);
        assert!(edits.is_empty(), "Should not transform when array is a variable");
    }

    #[test]
    fn test_variable_value() {
        let code = r#"<?php
in_array($needle, [$expected], true);
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$needle === $expected"));
    }

    #[test]
    fn test_function_call_in_array() {
        let code = r#"<?php
in_array(strtolower($type), ['test'], true);
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("strtolower($type) === 'test'"));
    }

    #[test]
    fn test_case_insensitive() {
        let code = r#"<?php
IN_ARRAY($x, ['y'], TRUE);
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$x === 'y'"));
    }
}
