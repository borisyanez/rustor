//! Rule: Convert array_push($arr, $val) to $arr[] = $val
//!
//! This transformation improves performance by avoiding function call overhead.
//!
//! Only transforms standalone array_push calls (not ones where return value is used).

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for array_push calls that can be simplified
pub fn check_array_push<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ArrayPushVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ArrayPushVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ArrayPushVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        // Only check expression statements for standalone array_push calls
        if let Statement::Expression(expr_stmt) = stmt {
            self.check_expression(&expr_stmt.expression);
        }
        true // Continue traversal
    }

    // Don't traverse into expressions - we only want standalone calls
    fn visit_expression(&mut self, _expr: &Expression<'a>, _source: &str) -> bool {
        false
    }
}

impl<'s> ArrayPushVisitor<'s> {
    fn check_expression(&mut self, expr: &Expression<'_>) {
        if let Expression::Call(call) = expr {
            if let Call::Function(func_call) = call {
                if let Expression::Identifier(ident) = func_call.function {
                    let name_span = ident.span();
                    let name =
                        &self.source[name_span.start.offset as usize..name_span.end.offset as usize];

                    if name.eq_ignore_ascii_case("array_push") {
                        let arg_list: Vec<_> = func_call.argument_list.arguments.iter().collect();

                        // Only handle simple case: exactly 2 arguments, neither unpacked
                        // array_push($arr, ...$vals) cannot be converted to $arr[] = ...$vals
                        if arg_list.len() == 2
                            && !arg_list[0].is_unpacked()
                            && !arg_list[1].is_unpacked()
                        {
                            let arr_span = arg_list[0].span();
                            let val_span = arg_list[1].span();

                            let arr_code = &self.source
                                [arr_span.start.offset as usize..arr_span.end.offset as usize];
                            let val_code = &self.source
                                [val_span.start.offset as usize..val_span.end.offset as usize];

                            let replacement = format!("{}[] = {}", arr_code, val_code);

                            self.edits.push(Edit::new(
                                call.span(),
                                replacement,
                                "Replace array_push() with short syntax for better performance",
                            ));
                        }
                    }
                }
            }
        }
    }
}

// Rule trait implementation
use crate::registry::Rule;

/// Rule struct for array_push transformation
pub struct ArrayPushRule;

impl Rule for ArrayPushRule {
    fn name(&self) -> &'static str {
        "array_push"
    }

    fn description(&self) -> &'static str {
        "Convert array_push($arr, $val) to $arr[] = $val"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_array_push(program, source)
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
        check_array_push(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_array_push() {
        let source = "<?php array_push($arr, $val);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $arr[] = $val;");
    }

    #[test]
    fn test_array_push_with_string() {
        let source = "<?php array_push($items, 'hello');";
        assert_eq!(transform(source), "<?php $items[] = 'hello';");
    }

    #[test]
    fn test_array_push_with_number() {
        let source = "<?php array_push($nums, 42);";
        assert_eq!(transform(source), "<?php $nums[] = 42;");
    }

    #[test]
    fn test_array_push_with_function_call() {
        let source = "<?php array_push($results, getValue());";
        assert_eq!(transform(source), "<?php $results[] = getValue();");
    }

    #[test]
    fn test_array_push_with_array_access() {
        let source = "<?php array_push($data['key'], $value);";
        assert_eq!(transform(source), "<?php $data['key'][] = $value;");
    }

    #[test]
    fn test_multiple_array_push_calls() {
        let source = "<?php array_push($a, 1); array_push($b, 2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a[] = 1; $b[] = 2;");
    }

    // ==================== Multi-value Skip Tests ====================

    #[test]
    fn test_skip_three_args() {
        let source = "<?php array_push($arr, 'one', 'two');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip 3-arg array_push");
    }

    #[test]
    fn test_skip_four_args() {
        let source = "<?php array_push($arr, 1, 2, 3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip 4-arg array_push");
    }

    #[test]
    fn test_skip_many_args() {
        let source = "<?php array_push($arr, $a, $b, $c, $d, $e);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip multi-arg array_push");
    }

    #[test]
    fn test_skip_variadic_spread() {
        // array_push($arr, ...$vals) cannot become $arr[] = ...$vals (invalid syntax)
        let source = "<?php array_push($arr, ...$values);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip variadic spread argument");
    }

    #[test]
    fn test_mixed_two_and_multi_args() {
        let source = r#"<?php
array_push($arr, 'single');
array_push($arr, 'one', 'two');
array_push($arr, 'another');
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2, "Should only transform 2-arg calls");
    }

    // ==================== Return Value Skip Tests ====================

    #[test]
    fn test_skip_assignment() {
        let source = "<?php $count = array_push($arr, $val);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip when return value assigned");
    }

    #[test]
    fn test_skip_in_condition() {
        let source = "<?php if (array_push($arr, $val) > 5) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip when used in condition");
    }

    #[test]
    fn test_skip_in_echo() {
        let source = "<?php echo array_push($arr, $val);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip when used in echo");
    }

    #[test]
    fn test_skip_in_return() {
        let source = "<?php return array_push($arr, $val);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip when used in return");
    }

    #[test]
    fn test_skip_as_function_argument() {
        let source = "<?php doSomething(array_push($arr, $val));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0, "Should skip when used as function arg");
    }

    // ==================== Nested Statement Tests ====================

    #[test]
    fn test_inside_if() {
        let source = r#"<?php
if ($condition) {
    array_push($arr, $val);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_else() {
        let source = r#"<?php
if ($x) {
    echo "x";
} else {
    array_push($arr, $val);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_elseif() {
        let source = r#"<?php
if ($x) {
    echo "x";
} elseif ($y) {
    array_push($arr, $val);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_for_loop() {
        let source = r#"<?php
for ($i = 0; $i < 10; $i++) {
    array_push($items, $i);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_foreach() {
        let source = r#"<?php
foreach ($items as $item) {
    array_push($result, $item);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_while() {
        let source = r#"<?php
while ($running) {
    array_push($log, 'tick');
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_do_while() {
        let source = r#"<?php
do {
    array_push($items, fetch());
} while ($hasMore);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_switch() {
        let source = r#"<?php
switch ($type) {
    case 'a':
        array_push($arr, 'found a');
        break;
    case 'b':
        array_push($arr, 'found b');
        break;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_inside_try_catch() {
        let source = r#"<?php
try {
    array_push($results, process());
} catch (Exception $e) {
    array_push($errors, $e->getMessage());
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_inside_finally() {
        let source = r#"<?php
try {
    risky();
} finally {
    array_push($log, 'done');
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Class/Method Tests ====================

    #[test]
    fn test_inside_class_method() {
        let source = r#"<?php
class Foo {
    public function bar() {
        array_push($this->items, $value);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_static_method() {
        let source = r#"<?php
class Foo {
    public static function bar() {
        array_push(self::$items, $value);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_trait_method() {
        let source = r#"<?php
trait MyTrait {
    public function process() {
        array_push($this->data, $item);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_abstract_method() {
        let source = r#"<?php
abstract class Foo {
    abstract public function bar();
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Function/Namespace Tests ====================

    #[test]
    fn test_inside_function() {
        let source = r#"<?php
function myFunc() {
    array_push($arr, $value);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_namespace() {
        let source = r#"<?php
namespace App\Service;

array_push($items, $value);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_inside_braced_namespace() {
        let source = r#"<?php
namespace App\Service {
    array_push($items, $value);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Case Insensitivity Tests ====================

    #[test]
    fn test_uppercase_array_push() {
        let source = "<?php ARRAY_PUSH($arr, $val);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_case_array_push() {
        let source = "<?php Array_Push($arr, $val);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_no_php_tag() {
        let source = "array_push($arr, $val);";
        let edits = check_php(source);
        // Without <?php tag, this is just text, not PHP
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_empty_file() {
        let source = "<?php ";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_deeply_nested() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($x) {
            foreach ($items as $item) {
                try {
                    array_push($this->results, $item);
                } catch (Exception $e) {
                    // skip
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
