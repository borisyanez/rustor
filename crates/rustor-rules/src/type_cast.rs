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
use rustor_core::Edit;

/// Check a parsed PHP program for type conversion functions that can use cast syntax
pub fn check_type_cast<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
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
            if let Some(edit) = try_transform_type_cast(expr, source) {
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
                // intval($x, $base) and floatval with precision should not be transformed
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
        // intval with base argument should NOT be transformed
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
        // doubleval is an alias for floatval
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
