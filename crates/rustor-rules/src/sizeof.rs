//! Rule: Convert sizeof($x) to count($x)
//!
//! sizeof() is an alias for count() in PHP. Using count() is preferred
//! as it's the canonical function name and more widely recognized.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for sizeof calls that can be replaced with count
pub fn check_sizeof<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SizeofVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SizeofVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SizeofVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_sizeof(expr, self.source) {
            self.edits.push(edit);
            return false; // Don't traverse children
        }
        true // Continue traversal
    }
}

/// Try to transform a sizeof() call, returning the Edit if successful
fn try_transform_sizeof(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if name.eq_ignore_ascii_case("sizeof") {
                let call_span = expr.span();
                let call_source =
                    &source[call_span.start.offset as usize..call_span.end.offset as usize];

                // Replace "sizeof" with "count" preserving case style
                let replacement = if name.chars().next().unwrap().is_uppercase() {
                    call_source.replacen(name, "Count", 1)
                } else {
                    call_source.replacen(name, "count", 1)
                };

                return Some(Edit::new(
                    call_span,
                    replacement,
                    "Replace sizeof() with count() (sizeof is an alias)",
                ));
            }
        }
    }
    None
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
        check_sizeof(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_sizeof() {
        let source = "<?php sizeof($arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php count($arr);");
    }

    #[test]
    fn test_sizeof_in_assignment() {
        let source = "<?php $len = sizeof($arr);";
        assert_eq!(transform(source), "<?php $len = count($arr);");
    }

    #[test]
    fn test_sizeof_in_condition() {
        let source = "<?php if (sizeof($arr) > 0) {}";
        assert_eq!(transform(source), "<?php if (count($arr) > 0) {}");
    }

    #[test]
    fn test_sizeof_in_echo() {
        let source = "<?php echo sizeof($items);";
        assert_eq!(transform(source), "<?php echo count($items);");
    }

    #[test]
    fn test_sizeof_in_return() {
        let source = "<?php return sizeof($data);";
        assert_eq!(transform(source), "<?php return count($data);");
    }

    #[test]
    fn test_sizeof_with_second_arg() {
        let source = "<?php sizeof($arr, COUNT_RECURSIVE);";
        assert_eq!(transform(source), "<?php count($arr, COUNT_RECURSIVE);");
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_sizeof() {
        let source = "<?php sizeof($a); sizeof($b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php count($a); count($b);");
    }

    #[test]
    fn test_sizeof_in_expression() {
        let source = "<?php $total = sizeof($a) + sizeof($b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $total = count($a) + count($b);");
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_sizeof_in_ternary() {
        let source = "<?php $result = sizeof($arr) > 0 ? 'has items' : 'empty';";
        assert_eq!(
            transform(source),
            "<?php $result = count($arr) > 0 ? 'has items' : 'empty';"
        );
    }

    #[test]
    fn test_sizeof_in_for_condition() {
        let source = "<?php for ($i = 0; $i < sizeof($arr); $i++) {}";
        assert_eq!(
            transform(source),
            "<?php for ($i = 0; $i < count($arr); $i++) {}"
        );
    }

    #[test]
    fn test_sizeof_in_while() {
        let source = "<?php while (sizeof($queue) > 0) {}";
        assert_eq!(transform(source), "<?php while (count($queue) > 0) {}");
    }

    #[test]
    fn test_sizeof_in_class_method() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return sizeof($this->items);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_sizeof_in_function() {
        let source = r#"<?php
function getLength($arr) {
    return sizeof($arr);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_sizeof() {
        let source = "<?php SIZEOF($arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php Count($arr);");
    }

    #[test]
    fn test_mixed_case_sizeof() {
        let source = "<?php SizeOf($arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_count_function() {
        let source = "<?php count($arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_sizeof($arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_sizeof_nested_call() {
        let source = "<?php sizeof(array_filter($arr));";
        assert_eq!(transform(source), "<?php count(array_filter($arr));");
    }

    #[test]
    fn test_sizeof_in_array() {
        let source = "<?php $lengths = [sizeof($a), sizeof($b)];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_sizeof_as_function_arg() {
        let source = "<?php doSomething(sizeof($arr));";
        assert_eq!(transform(source), "<?php doSomething(count($arr));");
    }

    #[test]
    fn test_deeply_nested_sizeof() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($condition) {
            foreach ($items as $item) {
                if (sizeof($item) > 0) {
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
