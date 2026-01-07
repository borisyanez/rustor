//! Rule: Convert sizeof($x) to count($x)
//!
//! sizeof() is an alias for count() in PHP. Using count() is preferred
//! as it's the canonical function name and more widely recognized.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

/// Check a parsed PHP program for sizeof calls that can be replaced with count
pub fn check_sizeof<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut edits = Vec::new();

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
            check_expression(&if_stmt.condition, source, edits);
            check_if_body(&if_stmt.body, source, edits);
        }
        Statement::Foreach(foreach) => {
            check_expression(&foreach.expression, source, edits);
            check_foreach_body(&foreach.body, source, edits);
        }
        Statement::For(for_stmt) => {
            for expr in for_stmt.initializations.iter() {
                check_expression(expr, source, edits);
            }
            for expr in for_stmt.conditions.iter() {
                check_expression(expr, source, edits);
            }
            for expr in for_stmt.increments.iter() {
                check_expression(expr, source, edits);
            }
            check_for_body(&for_stmt.body, source, edits);
        }
        Statement::While(while_stmt) => {
            check_expression(&while_stmt.condition, source, edits);
            check_while_body(&while_stmt.body, source, edits);
        }
        Statement::DoWhile(do_while) => {
            check_statement(&do_while.statement, source, edits);
            check_expression(&do_while.condition, source, edits);
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
            match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        check_statement(inner, source, edits);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
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
            check_expression(&switch.expression, source, edits);
            check_switch_body(&switch.body, source, edits);
        }
        Statement::Return(ret) => {
            if let Some(expr) = &ret.value {
                check_expression(expr, source, edits);
            }
        }
        Statement::Echo(echo) => {
            for expr in echo.values.iter() {
                check_expression(expr, source, edits);
            }
        }
        _ => {}
    }
}

fn check_if_body<'a>(body: &IfBody<'a>, source: &str, edits: &mut Vec<Edit>) {
    match body {
        IfBody::Statement(stmt_body) => {
            check_statement(stmt_body.statement, source, edits);
            for else_if in stmt_body.else_if_clauses.iter() {
                check_expression(&else_if.condition, source, edits);
                check_statement(else_if.statement, source, edits);
            }
            if let Some(else_clause) = &stmt_body.else_clause {
                check_statement(else_clause.statement, source, edits);
            }
        }
        IfBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                check_statement(inner, source, edits);
            }
            for else_if in block.else_if_clauses.iter() {
                check_expression(&else_if.condition, source, edits);
                for inner in else_if.statements.iter() {
                    check_statement(inner, source, edits);
                }
            }
            if let Some(else_clause) = &block.else_clause {
                for inner in else_clause.statements.iter() {
                    check_statement(inner, source, edits);
                }
            }
        }
    }
}

fn check_foreach_body<'a>(body: &ForeachBody<'a>, source: &str, edits: &mut Vec<Edit>) {
    match body {
        ForeachBody::Statement(stmt) => {
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
    match expr {
        Expression::Call(call) => {
            if let Some(edit) = try_transform_sizeof(expr, source) {
                edits.push(edit);
                return;
            }
            // Recurse into arguments
            if let Call::Function(func_call) = call {
                for arg in func_call.argument_list.arguments.iter() {
                    check_expression(arg.value(), source, edits);
                }
            }
        }
        Expression::UnaryPrefix(unary) => {
            check_expression(&unary.operand, source, edits);
        }
        Expression::Parenthesized(paren) => {
            check_expression(&paren.expression, source, edits);
        }
        Expression::Binary(binary) => {
            check_expression(&binary.lhs, source, edits);
            check_expression(&binary.rhs, source, edits);
        }
        Expression::Conditional(ternary) => {
            check_expression(&ternary.condition, source, edits);
            if let Some(if_expr) = &ternary.then {
                check_expression(if_expr, source, edits);
            }
            check_expression(&ternary.r#else, source, edits);
        }
        Expression::Assignment(assign) => {
            check_expression(&assign.lhs, source, edits);
            check_expression(&assign.rhs, source, edits);
        }
        Expression::ArrayAccess(access) => {
            check_expression(&access.array, source, edits);
            check_expression(&access.index, source, edits);
        }
        Expression::Array(arr) => {
            for elem in arr.elements.iter() {
                if let ArrayElement::KeyValue(kv) = elem {
                    check_expression(&kv.key, source, edits);
                    check_expression(&kv.value, source, edits);
                } else if let ArrayElement::Value(val) = elem {
                    check_expression(&val.value, source, edits);
                }
            }
        }
        _ => {}
    }
}

/// Try to transform a sizeof() call, returning the Edit if successful
fn try_transform_sizeof(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if name.eq_ignore_ascii_case("sizeof") {
                // Get the full call span and source
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
        // sizeof/count can have a second argument (COUNT_RECURSIVE)
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
        // Preserves uppercase style
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
        // Should not transform count() - it's already the canonical form
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
