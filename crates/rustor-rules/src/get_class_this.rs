//! Rule: Convert get_class($this) to $this::class (PHP 8.0+)
//!
//! PHP 8.0 introduced the ability to use ::class on objects, which is cleaner
//! and more consistent than using get_class().
//!
//! Example:
//! ```php
//! // Before
//! get_class($this)
//! get_class($obj)
//!
//! // After
//! $this::class
//! $obj::class
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check for get_class() calls that can be converted to ::class syntax
pub fn check_get_class_this<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = GetClassThisVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct GetClassThisVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for GetClassThisVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        self.check_expression(expr);
        true // Continue traversal
    }
}

impl<'s> GetClassThisVisitor<'s> {
    fn check_expression(&mut self, expr: &Expression<'_>) {
        if let Expression::Call(call) = expr {
            if let Call::Function(func_call) = call {
                if let Expression::Identifier(ident) = func_call.function {
                    let name_span = ident.span();
                    let name = &self.source[name_span.start.offset as usize..name_span.end.offset as usize];

                    if name.eq_ignore_ascii_case("get_class") {
                        let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

                        // get_class() with exactly one argument
                        if args.len() == 1 {
                            let arg = &args[0];
                            let arg_expr = arg.value();

                            // Check if argument is a simple variable (including $this)
                            if let Expression::Variable(var) = arg_expr {
                                let var_span = var.span();
                                let var_name = &self.source[var_span.start.offset as usize..var_span.end.offset as usize];

                                // Create replacement: $var::class
                                let replacement = format!("{}::class", var_name);

                                self.edits.push(Edit::new(
                                    call.span(),
                                    replacement.clone(),
                                    format!("Convert get_class({}) to {} (PHP 8.0+)", var_name, replacement),
                                ));
                            }
                        }
                        // get_class() with no arguments inside a class (returns current class)
                        // This is trickier - would need to track if we're inside a class
                        // For now, skip this case
                    }
                }
            }
        }
    }
}

pub struct GetClassThisRule;

impl Rule for GetClassThisRule {
    fn name(&self) -> &'static str {
        "get_class_this"
    }

    fn description(&self) -> &'static str {
        "Convert get_class($var) to $var::class (PHP 8.0+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_get_class_this(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
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
        check_get_class_this(program, code)
    }

    #[test]
    fn test_get_class_this() {
        let code = r#"<?php echo get_class($this);"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$this::class"));
    }

    #[test]
    fn test_get_class_variable() {
        let code = r#"<?php echo get_class($obj);"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$obj::class"));
    }

    #[test]
    fn test_get_class_in_condition() {
        let code = r#"<?php if (get_class($this) === 'Foo') {}"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_get_class_in_return() {
        let code = r#"<?php return get_class($instance);"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$instance::class"));
    }

    #[test]
    fn test_skip_get_class_no_args() {
        // get_class() with no args returns current class - skip for now
        let code = r#"<?php echo get_class();"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_get_class_two_args() {
        // get_class with 2 args is invalid but we shouldn't crash
        let code = r#"<?php echo get_class($a, $b);"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_multiple_get_class() {
        let code = r#"<?php
$a = get_class($this);
$b = get_class($other);
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_uppercase_get_class() {
        let code = r#"<?php echo GET_CLASS($this);"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_rule_metadata() {
        let rule = GetClassThisRule;
        assert_eq!(rule.name(), "get_class_this");
        assert_eq!(rule.category(), Category::Modernization);
        assert_eq!(rule.min_php_version(), Some(PhpVersion::Php80));
    }
}
