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
