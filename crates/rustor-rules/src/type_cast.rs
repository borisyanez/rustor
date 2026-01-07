//! Rule: Convert type conversion functions to cast syntax
//!
//! - strval($x)  → (string)$x
//! - intval($x)  → (int)$x
//! - floatval($x) → (float)$x
//!
//! Cast syntax is faster as it avoids function call overhead.
//! Note: intval() and floatval() with base/precision args are skipped.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for type conversion functions that can use cast syntax
pub fn check_type_cast<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = TypeCastVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct TypeCastVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for TypeCastVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_type_cast(expr, self.source) {
            self.edits.push(edit);
            return false; // Don't traverse children
        }
        true // Continue traversal
    }
}

/// Map function names to their cast equivalents
fn get_cast_for_function(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "strval" => Some("(string)"),
        "intval" => Some("(int)"),
        "floatval" | "doubleval" => Some("(float)"),
        "boolval" => Some("(bool)"),
        _ => None,
    }
}

/// Try to transform a type conversion function call, returning the Edit if successful
fn try_transform_type_cast(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if let Some(cast) = get_cast_for_function(name) {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

                // Only transform single-argument calls
                if args.len() == 1 {
                    let arg_span = args[0].span();
                    let arg_code =
                        &source[arg_span.start.offset as usize..arg_span.end.offset as usize];

                    let replacement = format!("{}{}", cast, arg_code);
                    let func_lower = name.to_lowercase();
                    let message = format!(
                        "Replace {}() with {} cast for better performance",
                        func_lower, cast
                    );

                    return Some(Edit::new(expr.span(), replacement, message));
                }
            }
        }
    }
    None
}

use crate::registry::Rule;

pub struct TypeCastRule;

impl Rule for TypeCastRule {
    fn name(&self) -> &'static str {
        "type_cast"
    }

    fn description(&self) -> &'static str {
        "Convert strval/intval/floatval/boolval to cast syntax"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_type_cast(program, source)
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
        check_type_cast(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== strval Tests ====================

    #[test]
    fn test_strval_simple() {
        let source = "<?php strval($x);";
        assert_eq!(transform(source), "<?php (string)$x;");
    }

    #[test]
    fn test_strval_in_assignment() {
        let source = "<?php $s = strval($num);";
        assert_eq!(transform(source), "<?php $s = (string)$num;");
    }

    #[test]
    fn test_strval_in_concat() {
        let source = "<?php echo 'Value: ' . strval($x);";
        assert_eq!(transform(source), "<?php echo 'Value: ' . (string)$x;");
    }

    #[test]
    fn test_strval_uppercase() {
        let source = "<?php STRVAL($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php (string)$x;");
    }

    // ==================== intval Tests ====================

    #[test]
    fn test_intval_simple() {
        let source = "<?php intval($x);";
        assert_eq!(transform(source), "<?php (int)$x;");
    }

    #[test]
    fn test_intval_in_assignment() {
        let source = "<?php $i = intval($str);";
        assert_eq!(transform(source), "<?php $i = (int)$str;");
    }

    #[test]
    fn test_intval_in_condition() {
        let source = "<?php if (intval($x) > 0) {}";
        assert_eq!(transform(source), "<?php if ((int)$x > 0) {}");
    }

    #[test]
    fn test_intval_skip_with_base() {
        let source = "<?php intval($hex, 16);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip intval with base argument");
    }

    #[test]
    fn test_intval_uppercase() {
        let source = "<?php INTVAL($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== floatval Tests ====================

    #[test]
    fn test_floatval_simple() {
        let source = "<?php floatval($x);";
        assert_eq!(transform(source), "<?php (float)$x;");
    }

    #[test]
    fn test_floatval_in_assignment() {
        let source = "<?php $f = floatval($str);";
        assert_eq!(transform(source), "<?php $f = (float)$str;");
    }

    #[test]
    fn test_floatval_in_calculation() {
        let source = "<?php $result = floatval($a) + floatval($b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $result = (float)$a + (float)$b;");
    }

    #[test]
    fn test_doubleval_alias() {
        let source = "<?php doubleval($x);";
        assert_eq!(transform(source), "<?php (float)$x;");
    }

    // ==================== boolval Tests ====================

    #[test]
    fn test_boolval_simple() {
        let source = "<?php boolval($x);";
        assert_eq!(transform(source), "<?php (bool)$x;");
    }

    #[test]
    fn test_boolval_in_condition() {
        let source = "<?php if (boolval($x)) {}";
        assert_eq!(transform(source), "<?php if ((bool)$x) {}");
    }

    // ==================== Multiple Functions ====================

    #[test]
    fn test_multiple_different_casts() {
        let source = "<?php $s = strval($a); $i = intval($b); $f = floatval($c);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 3);
        assert_eq!(
            transform(source),
            "<?php $s = (string)$a; $i = (int)$b; $f = (float)$c;"
        );
    }

    #[test]
    fn test_nested_casts() {
        // Outer call is transformed; inner call remains (would need second pass)
        let source = "<?php intval(strval($x));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php (int)strval($x);");
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_in_function() {
        let source = r#"<?php
function convert($x) {
    return intval($x);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_class_method() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return strval($this->value);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_ternary() {
        let source = "<?php $x = $cond ? intval($a) : floatval($b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_in_array() {
        let source = "<?php $arr = [intval($a), strval($b)];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_intval($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->intval($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_with_expression_arg() {
        let source = "<?php intval($a + $b);";
        assert_eq!(transform(source), "<?php (int)$a + $b;");
    }

    #[test]
    fn test_with_function_call_arg() {
        let source = "<?php intval(getValue());";
        assert_eq!(transform(source), "<?php (int)getValue();");
    }

    #[test]
    fn test_deeply_nested() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($condition) {
            foreach ($items as $item) {
                $values[] = intval($item);
            }
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
