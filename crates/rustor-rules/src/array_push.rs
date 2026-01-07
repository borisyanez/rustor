//! Rule: Convert array_push($arr, $val) to $arr[] = $val
//!
//! This transformation improves performance by avoiding function call overhead.
//!
//! Phase 0: Minimal implementation to validate mago integration

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

/// Check a parsed PHP program for array_push calls that can be simplified
pub fn check_array_push<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut edits = Vec::new();

    // Iterate through all statements in the program
    for stmt in program.statements.iter() {
        check_statement(stmt, source, &mut edits);
    }

    edits
}

fn check_statement<'a>(stmt: &Statement<'a>, source: &str, edits: &mut Vec<Edit>) {
    match stmt {
        Statement::Expression(expr_stmt) => {
            check_expression(&expr_stmt.expression, source, edits);
        }
        Statement::Block(block) => {
            for inner in block.statements.iter() {
                check_statement(inner, source, edits);
            }
        }
        Statement::If(if_stmt) => {
            check_if_body(&if_stmt.body, source, edits);
        }
        Statement::Foreach(foreach) => {
            check_foreach_body(&foreach.body, source, edits);
        }
        Statement::For(for_stmt) => {
            check_for_body(&for_stmt.body, source, edits);
        }
        Statement::While(while_stmt) => {
            check_while_body(&while_stmt.body, source, edits);
        }
        Statement::DoWhile(do_while) => {
            check_statement(&do_while.statement, source, edits);
        }
        Statement::Class(class) => {
            for member in class.members.iter() {
                check_class_like_member(member, source, edits);
            }
        }
        Statement::Function(func) => {
            for inner in func.body.statements.iter() {
                check_statement(inner, source, edits);
            }
        }
        Statement::Trait(tr) => {
            for member in tr.members.iter() {
                check_class_like_member(member, source, edits);
            }
        }
        Statement::Namespace(ns) => {
            // Visit namespace body statements
            match &ns.body {
                mago_syntax::ast::NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        check_statement(inner, source, edits);
                    }
                }
                mago_syntax::ast::NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        check_statement(inner, source, edits);
                    }
                }
            }
        }
        Statement::Try(try_stmt) => {
            for inner in try_stmt.block.statements.iter() {
                check_statement(inner, source, edits);
            }
            for catch in try_stmt.catch_clauses.iter() {
                for inner in catch.block.statements.iter() {
                    check_statement(inner, source, edits);
                }
            }
            if let Some(finally) = &try_stmt.finally_clause {
                for inner in finally.block.statements.iter() {
                    check_statement(inner, source, edits);
                }
            }
        }
        Statement::Switch(switch) => {
            check_switch_body(&switch.body, source, edits);
        }
        // Skip other statement types for now
        _ => {}
    }
}

fn check_if_body<'a>(body: &IfBody<'a>, source: &str, edits: &mut Vec<Edit>) {
    match body {
        IfBody::Statement(stmt_body) => {
            // IfStatementBody has a statement field and else_if_clauses
            check_statement(stmt_body.statement, source, edits);
            // Also check else-if clauses
            for else_if in stmt_body.else_if_clauses.iter() {
                check_statement(else_if.statement, source, edits);
            }
            // Check else clause
            if let Some(else_clause) = &stmt_body.else_clause {
                check_statement(else_clause.statement, source, edits);
            }
        }
        IfBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                check_statement(inner, source, edits);
            }
        }
    }
}

fn check_foreach_body<'a>(body: &ForeachBody<'a>, source: &str, edits: &mut Vec<Edit>) {
    match body {
        ForeachBody::Statement(stmt) => {
            // ForeachBody::Statement directly contains &Statement
            check_statement(stmt, source, edits);
        }
        ForeachBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                check_statement(inner, source, edits);
            }
        }
    }
}

fn check_for_body<'a>(body: &ForBody<'a>, source: &str, edits: &mut Vec<Edit>) {
    match body {
        ForBody::Statement(stmt) => {
            // ForBody::Statement directly contains &Statement
            check_statement(stmt, source, edits);
        }
        ForBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                check_statement(inner, source, edits);
            }
        }
    }
}

fn check_while_body<'a>(body: &WhileBody<'a>, source: &str, edits: &mut Vec<Edit>) {
    match body {
        WhileBody::Statement(stmt) => {
            // WhileBody::Statement directly contains &Statement
            check_statement(stmt, source, edits);
        }
        WhileBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                check_statement(inner, source, edits);
            }
        }
    }
}

fn check_switch_body<'a>(body: &SwitchBody<'a>, source: &str, edits: &mut Vec<Edit>) {
    match body {
        SwitchBody::BraceDelimited(block) => {
            for case in block.cases.iter() {
                for stmt in case.statements().iter() {
                    check_statement(stmt, source, edits);
                }
            }
        }
        SwitchBody::ColonDelimited(block) => {
            for case in block.cases.iter() {
                for stmt in case.statements().iter() {
                    check_statement(stmt, source, edits);
                }
            }
        }
    }
}

fn check_class_like_member<'a>(member: &ClassLikeMember<'a>, source: &str, edits: &mut Vec<Edit>) {
    if let ClassLikeMember::Method(method) = member {
        // Method body can be Concrete (has statements) or Abstract
        match &method.body {
            MethodBody::Concrete(body) => {
                for inner in body.statements.iter() {
                    check_statement(inner, source, edits);
                }
            }
            MethodBody::Abstract(_) => {}
        }
    }
}

fn check_expression<'a>(expr: &Expression<'a>, source: &str, edits: &mut Vec<Edit>) {
    // Check if this is a function call
    if let Expression::Call(call) = expr {
        // We only care about function calls, not method calls
        if let Call::Function(func_call) = call {
            // Get the function being called
            if let Expression::Identifier(ident) = func_call.function {
                // Extract function name from source
                let name_span = ident.span();
                let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

                // Check for array_push
                if name.eq_ignore_ascii_case("array_push") {
                    let arg_list: Vec<_> = func_call.argument_list.arguments.iter().collect();

                    // Only handle simple case: exactly 2 arguments
                    if arg_list.len() == 2 {
                        let arr_span = arg_list[0].span();
                        let val_span = arg_list[1].span();

                        let arr_code =
                            &source[arr_span.start.offset as usize..arr_span.end.offset as usize];
                        let val_code =
                            &source[val_span.start.offset as usize..val_span.end.offset as usize];

                        // Generate replacement: $arr[] = $val
                        let replacement = format!("{}[] = {}", arr_code, val_code);

                        edits.push(Edit::new(
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

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    /// Helper to parse PHP and run the array_push rule
    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_array_push(program, source)
    }

    /// Helper to apply edits and return the result
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
