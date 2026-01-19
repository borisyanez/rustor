//! Rule: Convert settype() to cast syntax
//!
//! When settype() is used as a standalone statement, it can be replaced with
//! a type cast assignment which is more readable.
//!
//! Transformation:
//! - `settype($x, 'string')` → `$x = (string) $x`
//! - `settype($x, 'int')` → `$x = (int) $x`
//! - `settype($x, 'bool')` → `$x = (bool) $x`
//! - `settype($x, 'float')` → `$x = (float) $x`
//! - `settype($x, 'array')` → `$x = (array) $x`
//! - `settype($x, 'object')` → `$x = (object) $x`
//! - `settype($x, 'null')` → `$x = null`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for settype() calls
pub fn check_settype_to_cast<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SettypeToCastVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SettypeToCastVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SettypeToCastVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        // We only transform settype() when it's a standalone expression statement
        if let Statement::Expression(expr_stmt) = stmt {
            if let Expression::Call(Call::Function(func_call)) = expr_stmt.expression {
                if let Some(replacement) = try_transform_settype(func_call, self.source) {
                    self.edits.push(Edit::new(
                        expr_stmt.expression.span(),
                        replacement,
                        "Convert settype() to cast syntax",
                    ));
                    return false;
                }
            }
        }
        true
    }
}

/// Map type string to PHP cast syntax
fn get_cast_for_type(type_str: &str) -> Option<&'static str> {
    match type_str.to_lowercase().as_str() {
        "string" => Some("(string)"),
        "int" | "integer" => Some("(int)"),
        "bool" | "boolean" => Some("(bool)"),
        "float" | "double" => Some("(float)"),
        "array" => Some("(array)"),
        "object" => Some("(object)"),
        _ => None,
    }
}

/// Try to transform settype(), returning the replacement if successful
fn try_transform_settype(func_call: &FunctionCall<'_>, source: &str) -> Option<String> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !name.eq_ignore_ascii_case("settype") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // settype needs exactly 2 arguments
    if args.len() != 2 {
        return None;
    }

    // Get the variable (first argument)
    let var_span = args[0].span();
    let var_text = &source[var_span.start.offset as usize..var_span.end.offset as usize];

    // Get the type string (second argument) - must be a string literal
    let type_arg = args[1].value();
    let type_str = if let Expression::Literal(Literal::String(string_lit)) = type_arg {
        let string_span = string_lit.span();
        let string_content = &source[string_span.start.offset as usize..string_span.end.offset as usize];
        // Remove quotes
        string_content.trim_matches(|c| c == '\'' || c == '"')
    } else {
        return None;
    };

    // Handle null specially
    if type_str.eq_ignore_ascii_case("null") {
        return Some(format!("{} = null", var_text));
    }

    // Get cast syntax for the type
    let cast = get_cast_for_type(type_str)?;

    Some(format!("{} = {} {}", var_text, cast, var_text))
}

use crate::registry::{Category, Rule};

pub struct SettypeToCastRule;

impl Rule for SettypeToCastRule {
    fn name(&self) -> &'static str {
        "settype_to_cast"
    }

    fn description(&self) -> &'static str {
        "Convert settype() to cast syntax"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_settype_to_cast(program, source)
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
        check_settype_to_cast(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Types ====================

    #[test]
    fn test_string() {
        let source = "<?php settype($foo, 'string');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $foo = (string) $foo;");
    }

    #[test]
    fn test_int() {
        let source = "<?php settype($x, 'int');";
        assert_eq!(transform(source), "<?php $x = (int) $x;");
    }

    #[test]
    fn test_integer() {
        let source = "<?php settype($x, 'integer');";
        assert_eq!(transform(source), "<?php $x = (int) $x;");
    }

    #[test]
    fn test_bool() {
        let source = "<?php settype($x, 'bool');";
        assert_eq!(transform(source), "<?php $x = (bool) $x;");
    }

    #[test]
    fn test_boolean() {
        let source = "<?php settype($x, 'boolean');";
        assert_eq!(transform(source), "<?php $x = (bool) $x;");
    }

    #[test]
    fn test_float() {
        let source = "<?php settype($x, 'float');";
        assert_eq!(transform(source), "<?php $x = (float) $x;");
    }

    #[test]
    fn test_double() {
        let source = "<?php settype($x, 'double');";
        assert_eq!(transform(source), "<?php $x = (float) $x;");
    }

    #[test]
    fn test_array() {
        let source = "<?php settype($x, 'array');";
        assert_eq!(transform(source), "<?php $x = (array) $x;");
    }

    #[test]
    fn test_object() {
        let source = "<?php settype($x, 'object');";
        assert_eq!(transform(source), "<?php $x = (object) $x;");
    }

    #[test]
    fn test_null() {
        let source = "<?php settype($x, 'null');";
        assert_eq!(transform(source), "<?php $x = null;");
    }

    // ==================== Double Quoted ====================

    #[test]
    fn test_double_quoted_type() {
        let source = r#"<?php settype($foo, "string");"#;
        assert_eq!(transform(source), r#"<?php $foo = (string) $foo;"#);
    }

    // ==================== Case Insensitive ====================

    #[test]
    fn test_uppercase_function() {
        let source = "<?php SETTYPE($x, 'int');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_uppercase_type() {
        let source = "<?php settype($x, 'STRING');";
        assert_eq!(transform(source), "<?php $x = (string) $x;");
    }

    // ==================== Complex Variables ====================

    #[test]
    fn test_property() {
        let source = "<?php settype($obj->prop, 'int');";
        assert_eq!(transform(source), "<?php $obj->prop = (int) $obj->prop;");
    }

    #[test]
    fn test_array_access() {
        let source = "<?php settype($arr[0], 'string');";
        assert_eq!(transform(source), "<?php $arr[0] = (string) $arr[0];");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
settype($a, 'int');
settype($b, 'string');
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_unknown_type() {
        let source = "<?php settype($x, 'unknown');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_variable_type() {
        let source = "<?php settype($x, $type);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_in_assignment() {
        // When settype() is used in an expression context, we can't transform it
        // because settype() returns bool and the cast assignment doesn't
        let source = "<?php $result = settype($x, 'int');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_single_arg() {
        let source = "<?php settype($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->settype($x, 'int');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
