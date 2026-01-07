//! AST visitor for traversing PHP syntax trees
//!
//! Provides a trait-based visitor pattern that rules can implement.
//! Default implementations handle traversal; rules override specific methods.

use mago_syntax::ast::*;

/// Trait for visiting PHP AST nodes
///
/// Default implementations traverse child nodes. Override specific methods
/// to perform actions at those nodes.
pub trait Visitor<'a> {
    /// Called for each expression. Return `true` to continue traversal into children.
    fn visit_expression(&mut self, _expr: &Expression<'a>, _source: &str) -> bool {
        true
    }

    /// Called for each statement. Return `true` to continue traversal into children.
    fn visit_statement(&mut self, _stmt: &Statement<'a>, _source: &str) -> bool {
        true
    }

    /// Visit a program (entry point)
    fn visit_program(&mut self, program: &Program<'a>, source: &str) {
        for stmt in program.statements.iter() {
            self.traverse_statement(stmt, source);
        }
    }

    /// Traverse a statement and its children
    fn traverse_statement(&mut self, stmt: &Statement<'a>, source: &str) {
        if !self.visit_statement(stmt, source) {
            return;
        }

        match stmt {
            Statement::Expression(expr_stmt) => {
                self.traverse_expression(&expr_stmt.expression, source);
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.traverse_statement(inner, source);
                }
            }
            Statement::If(if_stmt) => {
                self.traverse_expression(&if_stmt.condition, source);
                self.traverse_if_body(&if_stmt.body, source);
            }
            Statement::Foreach(foreach) => {
                self.traverse_expression(&foreach.expression, source);
                self.traverse_foreach_body(&foreach.body, source);
            }
            Statement::For(for_stmt) => {
                for expr in for_stmt.initializations.iter() {
                    self.traverse_expression(expr, source);
                }
                for expr in for_stmt.conditions.iter() {
                    self.traverse_expression(expr, source);
                }
                for expr in for_stmt.increments.iter() {
                    self.traverse_expression(expr, source);
                }
                self.traverse_for_body(&for_stmt.body, source);
            }
            Statement::While(while_stmt) => {
                self.traverse_expression(&while_stmt.condition, source);
                self.traverse_while_body(&while_stmt.body, source);
            }
            Statement::DoWhile(do_while) => {
                self.traverse_statement(&do_while.statement, source);
                self.traverse_expression(&do_while.condition, source);
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    self.traverse_class_like_member(member, source);
                }
            }
            Statement::Function(func) => {
                for inner in func.body.statements.iter() {
                    self.traverse_statement(inner, source);
                }
            }
            Statement::Trait(tr) => {
                for member in tr.members.iter() {
                    self.traverse_class_like_member(member, source);
                }
            }
            Statement::Namespace(ns) => {
                match &ns.body {
                    NamespaceBody::Implicit(body) => {
                        for inner in body.statements.iter() {
                            self.traverse_statement(inner, source);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.traverse_statement(inner, source);
                        }
                    }
                }
            }
            Statement::Try(try_stmt) => {
                for inner in try_stmt.block.statements.iter() {
                    self.traverse_statement(inner, source);
                }
                for catch in try_stmt.catch_clauses.iter() {
                    for inner in catch.block.statements.iter() {
                        self.traverse_statement(inner, source);
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    for inner in finally.block.statements.iter() {
                        self.traverse_statement(inner, source);
                    }
                }
            }
            Statement::Switch(switch) => {
                self.traverse_expression(&switch.expression, source);
                self.traverse_switch_body(&switch.body, source);
            }
            Statement::Return(ret) => {
                if let Some(expr) = &ret.value {
                    self.traverse_expression(expr, source);
                }
            }
            Statement::Echo(echo) => {
                for expr in echo.values.iter() {
                    self.traverse_expression(expr, source);
                }
            }
            _ => {}
        }
    }

    /// Traverse an if body
    fn traverse_if_body(&mut self, body: &IfBody<'a>, source: &str) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.traverse_statement(stmt_body.statement, source);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.traverse_expression(&else_if.condition, source);
                    self.traverse_statement(else_if.statement, source);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.traverse_statement(else_clause.statement, source);
                }
            }
            IfBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.traverse_statement(inner, source);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.traverse_expression(&else_if.condition, source);
                    for inner in else_if.statements.iter() {
                        self.traverse_statement(inner, source);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        self.traverse_statement(inner, source);
                    }
                }
            }
        }
    }

    /// Traverse a foreach body
    fn traverse_foreach_body(&mut self, body: &ForeachBody<'a>, source: &str) {
        match body {
            ForeachBody::Statement(stmt) => {
                self.traverse_statement(stmt, source);
            }
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.traverse_statement(inner, source);
                }
            }
        }
    }

    /// Traverse a for body
    fn traverse_for_body(&mut self, body: &ForBody<'a>, source: &str) {
        match body {
            ForBody::Statement(stmt) => {
                self.traverse_statement(stmt, source);
            }
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.traverse_statement(inner, source);
                }
            }
        }
    }

    /// Traverse a while body
    fn traverse_while_body(&mut self, body: &WhileBody<'a>, source: &str) {
        match body {
            WhileBody::Statement(stmt) => {
                self.traverse_statement(stmt, source);
            }
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.traverse_statement(inner, source);
                }
            }
        }
    }

    /// Traverse a switch body
    fn traverse_switch_body(&mut self, body: &SwitchBody<'a>, source: &str) {
        match body {
            SwitchBody::BraceDelimited(block) => {
                for case in block.cases.iter() {
                    for stmt in case.statements().iter() {
                        self.traverse_statement(stmt, source);
                    }
                }
            }
            SwitchBody::ColonDelimited(block) => {
                for case in block.cases.iter() {
                    for stmt in case.statements().iter() {
                        self.traverse_statement(stmt, source);
                    }
                }
            }
        }
    }

    /// Traverse a class-like member
    fn traverse_class_like_member(&mut self, member: &ClassLikeMember<'a>, source: &str) {
        if let ClassLikeMember::Method(method) = member {
            match &method.body {
                MethodBody::Concrete(body) => {
                    for inner in body.statements.iter() {
                        self.traverse_statement(inner, source);
                    }
                }
                MethodBody::Abstract(_) => {}
            }
        }
    }

    /// Traverse an expression and its children
    fn traverse_expression(&mut self, expr: &Expression<'a>, source: &str) {
        if !self.visit_expression(expr, source) {
            return;
        }

        match expr {
            Expression::Call(call) => {
                if let Call::Function(func_call) = call {
                    for arg in func_call.argument_list.arguments.iter() {
                        self.traverse_expression(arg.value(), source);
                    }
                }
            }
            Expression::UnaryPrefix(unary) => {
                self.traverse_expression(&unary.operand, source);
            }
            Expression::Parenthesized(paren) => {
                self.traverse_expression(&paren.expression, source);
            }
            Expression::Binary(binary) => {
                self.traverse_expression(&binary.lhs, source);
                self.traverse_expression(&binary.rhs, source);
            }
            Expression::Conditional(ternary) => {
                self.traverse_expression(&ternary.condition, source);
                if let Some(if_expr) = &ternary.then {
                    self.traverse_expression(if_expr, source);
                }
                self.traverse_expression(&ternary.r#else, source);
            }
            Expression::Assignment(assign) => {
                self.traverse_expression(&assign.lhs, source);
                self.traverse_expression(&assign.rhs, source);
            }
            Expression::ArrayAccess(access) => {
                self.traverse_expression(&access.array, source);
                self.traverse_expression(&access.index, source);
            }
            Expression::Array(arr) => {
                for elem in arr.elements.iter() {
                    if let ArrayElement::KeyValue(kv) = elem {
                        self.traverse_expression(&kv.key, source);
                        self.traverse_expression(&kv.value, source);
                    } else if let ArrayElement::Value(val) = elem {
                        self.traverse_expression(&val.value, source);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Helper function to run a visitor on a program
pub fn visit<'a, V: Visitor<'a>>(visitor: &mut V, program: &Program<'a>, source: &str) {
    visitor.visit_program(program, source);
}
