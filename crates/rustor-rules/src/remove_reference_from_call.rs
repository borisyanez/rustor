//! Rule: Remove call-time pass-by-reference from function calls
//!
//! Since PHP 5.4, call-time pass-by-reference (using & in function calls) is removed.
//! The & must be in the function parameter definition, not in the call.
//!
//! Transformation:
//! - `strlen(&$one)` → `strlen($one)`
//! - `call(&$a, &$b)` → `call($a, $b)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for call-time pass-by-reference
pub fn check_remove_reference_from_call<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveReferenceFromCallVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveReferenceFromCallVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RemoveReferenceFromCallVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        match expr {
            Expression::Call(Call::Function(func_call)) => {
                for arg in func_call.argument_list.arguments.iter() {
                    self.check_arg(arg);
                }
            }
            Expression::Call(Call::Method(method_call)) => {
                for arg in method_call.argument_list.arguments.iter() {
                    self.check_arg(arg);
                }
            }
            Expression::Call(Call::StaticMethod(static_call)) => {
                for arg in static_call.argument_list.arguments.iter() {
                    self.check_arg(arg);
                }
            }
            _ => {}
        }
        true
    }
}

impl<'s> RemoveReferenceFromCallVisitor<'s> {
    fn check_arg(&mut self, arg: &Argument<'_>) {
        let arg_value = arg.value();

        // Check if the argument is a reference (UnaryPrefix with Reference operator)
        if let Expression::UnaryPrefix(unary) = arg_value {
            if matches!(unary.operator, UnaryPrefixOperator::Reference(_)) {
                // Remove the & prefix - replace &$var with $var
                let full_span = arg_value.span();
                let inner_span = unary.operand.span();
                let inner_text =
                    &self.source[inner_span.start.offset as usize..inner_span.end.offset as usize];

                self.edits.push(Edit::new(
                    full_span,
                    inner_text.to_string(),
                    "Remove call-time pass-by-reference",
                ));
            }
        }
    }
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct RemoveReferenceFromCallRule;

impl Rule for RemoveReferenceFromCallRule {
    fn name(&self) -> &'static str {
        "remove_reference_from_call"
    }

    fn description(&self) -> &'static str {
        "Remove call-time pass-by-reference from function calls"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_reference_from_call(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php54)
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
        check_remove_reference_from_call(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Function Calls ====================

    #[test]
    fn test_function_call() {
        let source = "<?php strlen(&$one);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php strlen($one);");
    }

    #[test]
    fn test_function_call_multiple_args() {
        let source = "<?php array_push(&$arr, &$value);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php array_push($arr, $value);");
    }

    #[test]
    fn test_function_call_mixed_args() {
        let source = "<?php func($a, &$b, $c);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php func($a, $b, $c);");
    }

    // ==================== Method Calls ====================

    #[test]
    fn test_method_call() {
        let source = "<?php $obj->method(&$value);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $obj->method($value);");
    }

    // ==================== Static Method Calls ====================

    #[test]
    fn test_static_method_call() {
        let source = "<?php Foo::bar(&$value);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php Foo::bar($value);");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_class() {
        let source = r#"<?php
final class SomeClass {
    public function run($one) {
        return strlen(&$one);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple_calls() {
        let source = r#"<?php
strlen(&$a);
count(&$b);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_no_reference() {
        let source = "<?php strlen($one);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_function_definition() {
        // Reference in parameter definition should not be removed
        let source = "<?php function foo(&$x) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
