//! Rule: Fix deprecated implode() argument order (PHP 7.4+)
//!
//! In PHP 7.4+, passing the array as the first argument to implode() is deprecated.
//!
//! Example:
//! ```php
//! // Before (deprecated)
//! implode($array, ', ');
//!
//! // After
//! implode(', ', $array);
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check for deprecated implode() argument order
pub fn check_implode_order<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ImplodeOrderVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ImplodeOrderVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ImplodeOrderVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        self.check_expression(expr);
        true // Continue traversal
    }
}

impl<'s> ImplodeOrderVisitor<'s> {
    fn check_expression(&mut self, expr: &Expression<'_>) {
        if let Expression::Call(call) = expr {
            if let Call::Function(func_call) = call {
                if let Expression::Identifier(ident) = func_call.function {
                    let name_span = ident.span();
                    let name = &self.source[name_span.start.offset as usize..name_span.end.offset as usize];

                    if name.eq_ignore_ascii_case("implode") {
                        let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

                        // Need exactly 2 arguments
                        if args.len() == 2 {
                            let first_arg = &args[0];
                            let second_arg = &args[1];

                            // Check if first argument is array-like and second is a string literal
                            let first_is_array_like = self.is_array_like(&first_arg.value());
                            let second_is_string_like = self.is_string_like(&second_arg.value());

                            if first_is_array_like && second_is_string_like {
                                // Deprecated order detected - swap arguments
                                let first_span = first_arg.span();
                                let second_span = second_arg.span();

                                let first_text = &self.source[first_span.start.offset as usize..first_span.end.offset as usize];
                                let second_text = &self.source[second_span.start.offset as usize..second_span.end.offset as usize];

                                // Create new text with swapped arguments
                                let new_text = format!("{}, {}", second_text, first_text);

                                // Span from first arg to second arg (use file_id from first span)
                                let full_span = mago_span::Span::new(
                                    first_span.file_id,
                                    first_span.start,
                                    second_span.end,
                                );

                                self.edits.push(Edit::new(
                                    full_span,
                                    new_text,
                                    format!(
                                        "Fix deprecated implode() argument order: implode({}, {}) -> implode({}, {})",
                                        first_text, second_text, second_text, first_text
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    fn is_array_like(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Variable(_)
                | Expression::Array(_)
                | Expression::ArrayAccess(_)
                | Expression::Call(_)
                | Expression::Access(_)
        )
    }

    fn is_string_like(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Literal(Literal::String(_)))
    }
}

pub struct ImplodeOrderRule;

impl Rule for ImplodeOrderRule {
    fn name(&self) -> &'static str {
        "implode_order"
    }

    fn description(&self) -> &'static str {
        "Fix deprecated implode() argument order (array first is deprecated in PHP 7.4+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_implode_order(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php74)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_check(code: &str) -> Vec<Edit> {
        use bumpalo::Bump;
        use mago_database::file::FileId;

        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, code);
        check_implode_order(program, code)
    }

    #[test]
    fn test_deprecated_order() {
        let code = r#"<?php implode($array, ', ');"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].message.contains("deprecated"));
    }

    #[test]
    fn test_correct_order() {
        let code = r#"<?php implode(', ', $array);"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_deprecated_with_function_call() {
        let code = r#"<?php implode(getArray(), '-');"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_single_arg() {
        // Single arg implode is valid
        let code = r#"<?php implode($array);"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_correct_variable_order() {
        // If second arg is not a string literal, don't flag it
        let code = r#"<?php implode($sep, $array);"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_array_access_first() {
        let code = r#"<?php implode($data['items'], ':');"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_rule_metadata() {
        let rule = ImplodeOrderRule;
        assert_eq!(rule.name(), "implode_order");
        assert_eq!(rule.category(), Category::Compatibility);
        assert_eq!(rule.min_php_version(), Some(PhpVersion::Php74));
    }

    #[test]
    fn test_inside_function() {
        let code = r#"<?php
function foo() {
    return implode($arr, '-');
}
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_as_argument() {
        let code = r#"<?php echo implode($values, ', ');"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }
}
