//! Rule: Convert is_null($x) to $x === null
//!
//! This transformation improves performance by avoiding function call overhead.
//! Also handles negation: !is_null($x) → $x !== null

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

/// Check a parsed PHP program for is_null calls that can be simplified
pub fn check_is_null<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
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
            // Check condition
            check_expression(&if_stmt.condition, source, edits);
            check_if_body(&if_stmt.body, source, edits);
        }
        Statement::Foreach(foreach) => {
            check_expression(&foreach.expression, source, edits);
            check_foreach_body(&foreach.body, source, edits);
        }
        Statement::For(for_stmt) => {
            // Check initializations, conditions, and increments
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
        // Handle !is_null($x) → $x !== null
        Expression::UnaryPrefix(unary) => {
            if let UnaryPrefixOperator::Not(_) = &unary.operator {
                if let Some(edit) = try_transform_is_null(&unary.operand, source, true) {
                    // Replace the entire !is_null(...) expression
                    edits.push(Edit::new(
                        expr.span(),
                        edit,
                        "Replace !is_null() with !== null for better performance",
                    ));
                    return;
                }
            }
            // Recurse into operand
            check_expression(&unary.operand, source, edits);
        }

        // Handle is_null($x) → $x === null
        Expression::Call(call) => {
            if let Some(edit) = try_transform_is_null(expr, source, false) {
                edits.push(Edit::new(
                    expr.span(),
                    edit,
                    "Replace is_null() with === null for better performance",
                ));
                return;
            }
            // Recurse into arguments
            if let Call::Function(func_call) = call {
                for arg in func_call.argument_list.arguments.iter() {
                    check_expression(arg.value(), source, edits);
                }
            }
        }

        // Recurse into other expression types
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

/// Try to transform an is_null() call, returning the replacement string if successful
fn try_transform_is_null(expr: &Expression<'_>, source: &str, negated: bool) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if name.eq_ignore_ascii_case("is_null") {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

                // is_null takes exactly 1 argument
                if args.len() == 1 {
                    let arg_span = args[0].span();
                    let arg_code =
                        &source[arg_span.start.offset as usize..arg_span.end.offset as usize];

                    let operator = if negated { "!==" } else { "===" };
                    return Some(format!("{} {} null", arg_code, operator));
                }
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
        check_is_null(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_is_null() {
        let source = "<?php is_null($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x === null;");
    }

    #[test]
    fn test_is_null_in_condition() {
        let source = "<?php if (is_null($x)) {}";
        assert_eq!(transform(source), "<?php if ($x === null) {}");
    }

    #[test]
    fn test_is_null_in_assignment() {
        let source = "<?php $result = is_null($x);";
        assert_eq!(transform(source), "<?php $result = $x === null;");
    }

    #[test]
    fn test_is_null_with_array_access() {
        let source = "<?php is_null($arr['key']);";
        assert_eq!(transform(source), "<?php $arr['key'] === null;");
    }

    #[test]
    fn test_is_null_with_method_call() {
        let source = "<?php is_null($obj->getValue());";
        assert_eq!(transform(source), "<?php $obj->getValue() === null;");
    }

    // ==================== Negation Tests ====================

    #[test]
    fn test_negated_is_null() {
        let source = "<?php !is_null($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $x !== null;");
    }

    #[test]
    fn test_negated_is_null_in_condition() {
        let source = "<?php if (!is_null($x)) {}";
        assert_eq!(transform(source), "<?php if ($x !== null) {}");
    }

    #[test]
    fn test_negated_is_null_in_assignment() {
        let source = "<?php $result = !is_null($x);";
        assert_eq!(transform(source), "<?php $result = $x !== null;");
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_is_null() {
        let source = "<?php is_null($a); is_null($b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(transform(source), "<?php $a === null; $b === null;");
    }

    #[test]
    fn test_mixed_is_null_and_negated() {
        let source = "<?php if (is_null($a) || !is_null($b)) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(
            transform(source),
            "<?php if ($a === null || $b !== null) {}"
        );
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_is_null_in_ternary() {
        let source = "<?php $result = is_null($x) ? 'yes' : 'no';";
        assert_eq!(transform(source), "<?php $result = $x === null ? 'yes' : 'no';");
    }

    #[test]
    fn test_is_null_in_binary_expression() {
        let source = "<?php if (is_null($x) && $y > 0) {}";
        assert_eq!(transform(source), "<?php if ($x === null && $y > 0) {}");
    }

    #[test]
    fn test_is_null_in_return() {
        let source = "<?php return is_null($x);";
        assert_eq!(transform(source), "<?php return $x === null;");
    }

    #[test]
    fn test_is_null_in_echo() {
        let source = "<?php echo is_null($x) ? 'null' : 'not null';";
        assert_eq!(transform(source), "<?php echo $x === null ? 'null' : 'not null';");
    }

    // ==================== Statement Context Tests ====================

    #[test]
    fn test_is_null_in_while_condition() {
        let source = "<?php while (!is_null($item = next($arr))) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_is_null_in_for_condition() {
        let source = "<?php for ($i = 0; !is_null($arr[$i]); $i++) {}";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_is_null_in_switch() {
        let source = r#"<?php
switch ($type) {
    case 'foo':
        if (is_null($x)) { break; }
        break;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_is_null_in_class_method() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return is_null($this->value);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_uppercase_is_null() {
        let source = "<?php IS_NULL($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_case_is_null() {
        let source = "<?php Is_Null($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_wrong_arg_count() {
        // is_null with wrong number of args should be skipped
        let source = "<?php is_null();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_is_null($x);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_is_null_in_array() {
        let source = "<?php $arr = [is_null($a), !is_null($b)];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_is_null_in_short_ternary() {
        // Short ternary (Elvis operator): is_null($x) ?: 'default'
        let source = "<?php $result = is_null($x) ?: 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_deeply_nested_is_null() {
        let source = r#"<?php
class Foo {
    public function bar() {
        if ($condition) {
            foreach ($items as $item) {
                if (!is_null($item->value)) {
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
