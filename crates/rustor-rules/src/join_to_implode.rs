//! Rule: Convert join() to implode()
//!
//! join() is an alias for implode() in PHP. Using implode() is preferred
//! as it's the canonical function name and more widely recognized.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for join calls that can be replaced with implode
pub fn check_join_to_implode<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = JoinToImplodeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct JoinToImplodeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for JoinToImplodeVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_join(expr, self.source) {
            self.edits.push(edit);
            return false; // Don't traverse children
        }
        true // Continue traversal
    }
}

/// Try to transform a join() call, returning the Edit if successful
fn try_transform_join(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if name.eq_ignore_ascii_case("join") {
                let call_span = expr.span();
                let call_source =
                    &source[call_span.start.offset as usize..call_span.end.offset as usize];

                // Replace "join" with "implode" preserving case style
                let replacement = if name.chars().next().unwrap().is_uppercase() {
                    call_source.replacen(name, "Implode", 1)
                } else {
                    call_source.replacen(name, "implode", 1)
                };

                return Some(Edit::new(
                    call_span,
                    replacement,
                    "Replace join() with implode() (join is an alias)",
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
        check_join_to_implode(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_join() {
        let source = "<?php join(',', $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php implode(',', $arr);");
    }

    #[test]
    fn test_join_in_assignment() {
        let source = "<?php $str = join(',', $arr);";
        assert_eq!(transform(source), "<?php $str = implode(',', $arr);");
    }

    #[test]
    fn test_join_in_echo() {
        let source = "<?php echo join(', ', $items);";
        assert_eq!(transform(source), "<?php echo implode(', ', $items);");
    }

    #[test]
    fn test_join_in_return() {
        let source = "<?php return join('-', $parts);";
        assert_eq!(transform(source), "<?php return implode('-', $parts);");
    }

    #[test]
    fn test_join_single_arg() {
        // Single argument form (deprecated but valid)
        let source = "<?php join($arr);";
        assert_eq!(transform(source), "<?php implode($arr);");
    }

    #[test]
    fn test_join_with_empty_string() {
        let source = "<?php join('', $arr);";
        assert_eq!(transform(source), "<?php implode('', $arr);");
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_join() {
        let source = "<?php join(',', $a); join('-', $b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php implode(',', $a); implode('-', $b);");
    }

    #[test]
    fn test_join_in_expression() {
        let source = "<?php $result = join(',', $a) . join('-', $b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $result = implode(',', $a) . implode('-', $b);");
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_join_in_function() {
        let source = r#"<?php
function formatList($items) {
    return join(', ', $items);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_join_in_class_method() {
        let source = r#"<?php
class Formatter {
    public function format($items) {
        return join(', ', $items);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_join_in_ternary() {
        let source = "<?php $result = $items ? join(',', $items) : '';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_join_in_array() {
        let source = "<?php $arr = [join(',', $a), join('-', $b)];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_join_as_function_arg() {
        let source = "<?php doSomething(join(',', $arr));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_join() {
        let source = "<?php JOIN(',', $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php Implode(',', $arr);");
    }

    #[test]
    fn test_mixed_case_join() {
        let source = "<?php Join(',', $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_implode() {
        // Already using implode
        let source = "<?php implode(',', $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_join(',', $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->join(',', $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_static_method() {
        let source = "<?php Foo::join(',', $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_join_with_variable_separator() {
        let source = "<?php join($sep, $arr);";
        assert_eq!(transform(source), "<?php implode($sep, $arr);");
    }

    #[test]
    fn test_join_with_function_call_separator() {
        let source = "<?php join(getSeparator(), $arr);";
        assert_eq!(transform(source), "<?php implode(getSeparator(), $arr);");
    }

    #[test]
    fn test_join_nested_in_other_call() {
        let source = "<?php strlen(join(',', $arr));";
        assert_eq!(transform(source), "<?php strlen(implode(',', $arr));");
    }

    #[test]
    fn test_deeply_nested() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($condition) {
            foreach ($items as $item) {
                $result = join(',', $item);
            }
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
