//! Check for boolean operations that always evaluate to false (Level 4)
//!
//! Detects boolean negation of always-true expressions and other impossible boolean operations.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::path::PathBuf;

/// Checks for boolean operations that are always false
pub struct AlwaysFalseBooleanCheck;

impl Check for AlwaysFalseBooleanCheck {
    fn id(&self) -> &'static str {
        "booleanNot.alwaysFalse"
    }

    fn description(&self) -> &'static str {
        "Detects boolean negation of always-true expressions"
    }

    fn level(&self) -> u8 {
        4
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = BooleanAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };

        analyzer.analyze_program(program);
        analyzer.issues
    }
}

struct BooleanAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

impl<'s> BooleanAnalyzer<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn get_line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// Check if an expression is always truthy
    fn is_always_truthy<'a>(&self, expr: &Expression<'a>) -> bool {
        match expr {
            // Literal true
            Expression::Literal(Literal::True(_)) => true,

            // Non-zero integers
            Expression::Literal(Literal::Integer(int)) => {
                let text = self.get_span_text(&int.span);
                // Parse as i64, check if non-zero
                if let Ok(val) = text.parse::<i64>() {
                    val != 0
                } else {
                    false
                }
            }

            // Non-empty strings
            Expression::Literal(Literal::String(s)) => {
                let text = self.get_span_text(&s.span());
                // Remove quotes and check if non-empty
                let content = text.trim_matches(|c| c == '"' || c == '\'');
                !content.is_empty()
            }

            // Non-zero floats
            Expression::Literal(Literal::Float(f)) => {
                let text = self.get_span_text(&f.span);
                if let Ok(val) = text.parse::<f64>() {
                    val != 0.0
                } else {
                    false
                }
            }

            // Parenthesized expressions
            Expression::Parenthesized(p) => {
                self.is_always_truthy(&p.expression)
            }

            // Equality comparisons with same literals
            Expression::Binary(binary) => {
                matches!(
                    binary.operator,
                    BinaryOperator::Equal(_) | BinaryOperator::Identical(_)
                ) && self.are_same_literals(&binary.lhs, &binary.rhs)
            }

            _ => false,
        }
    }

    /// Check if an expression is always falsy
    fn is_always_falsy<'a>(&self, expr: &Expression<'a>) -> bool {
        match expr {
            // Literal false
            Expression::Literal(Literal::False(_)) => true,

            // Zero integer
            Expression::Literal(Literal::Integer(int)) => {
                let text = self.get_span_text(&int.span);
                text == "0"
            }

            // Empty string
            Expression::Literal(Literal::String(s)) => {
                let text = self.get_span_text(&s.span());
                let content = text.trim_matches(|c| c == '"' || c == '\'');
                content.is_empty()
            }

            // Zero float
            Expression::Literal(Literal::Float(f)) => {
                let text = self.get_span_text(&f.span);
                text == "0.0" || text == "0."
            }

            // Parenthesized expressions
            Expression::Parenthesized(p) => {
                self.is_always_falsy(&p.expression)
            }

            // Inequality comparisons with same literals
            Expression::Binary(binary) => {
                matches!(
                    binary.operator,
                    BinaryOperator::NotEqual(_) | BinaryOperator::NotIdentical(_)
                ) && self.are_same_literals(&binary.lhs, &binary.rhs)
            }

            _ => false,
        }
    }

    /// Check if two expressions are the same literal value
    fn are_same_literals<'a>(&self, lhs: &Expression<'a>, rhs: &Expression<'a>) -> bool {
        match (lhs, rhs) {
            (Expression::Literal(Literal::Integer(l)), Expression::Literal(Literal::Integer(r))) => {
                self.get_span_text(&l.span) == self.get_span_text(&r.span)
            }
            (Expression::Literal(Literal::String(l)), Expression::Literal(Literal::String(r))) => {
                self.get_span_text(&l.span()) == self.get_span_text(&r.span())
            }
            (Expression::Literal(Literal::True(_)), Expression::Literal(Literal::True(_))) => true,
            (Expression::Literal(Literal::False(_)), Expression::Literal(Literal::False(_))) => true,
            _ => false,
        }
    }

    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.analyze_expression(&expr_stmt.expression);
            }
            Statement::If(if_stmt) => {
                self.analyze_expression(&if_stmt.condition);
                self.analyze_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.analyze_expression(&while_stmt.condition);
                self.analyze_while_body(&while_stmt.body);
            }
            Statement::For(for_stmt) => {
                for init in for_stmt.initializations.iter() {
                    self.analyze_expression(init);
                }
                for cond in for_stmt.conditions.iter() {
                    self.analyze_expression(cond);
                }
                for inc in for_stmt.increments.iter() {
                    self.analyze_expression(inc);
                }
                self.analyze_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.analyze_expression(&foreach.expression);
                self.analyze_foreach_body(&foreach.body);
            }
            Statement::Function(func) => {
                for stmt in func.body.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            for stmt in body.statements.iter() {
                                self.analyze_statement(stmt);
                            }
                        }
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.analyze_expression(value);
                }
            }
            Statement::Echo(echo) => {
                for value in echo.values.iter() {
                    self.analyze_expression(value);
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            },
            _ => {}
        }
    }

    fn analyze_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // Check boolean NOT operator
            Expression::UnaryPrefix(unary) => {
                if matches!(unary.operator, UnaryPrefixOperator::Not(_)) {
                    // Check if negating an always-truthy value
                    if self.is_always_truthy(&unary.operand) {
                        let (line, col) = self.get_line_col(expr.span().start.offset as usize);
                        let expr_text = self.get_span_text(&expr.span());
                        self.issues.push(
                            Issue::error(
                                "booleanNot.alwaysFalse",
                                format!(
                                    "Negated boolean expression is always false: {}",
                                    expr_text
                                ),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("booleanNot.alwaysFalse"),
                        );
                    }
                }
                self.analyze_expression(&unary.operand);
            }
            Expression::Binary(binary) => {
                self.analyze_expression(&binary.lhs);
                self.analyze_expression(&binary.rhs);
            }
            Expression::UnaryPostfix(postfix) => {
                self.analyze_expression(&postfix.operand);
            }
            Expression::Conditional(cond) => {
                self.analyze_expression(&cond.condition);
                if let Some(then) = &cond.then {
                    self.analyze_expression(then);
                }
                self.analyze_expression(&cond.r#else);
            }
            Expression::Assignment(assign) => {
                self.analyze_expression(&assign.lhs);
                self.analyze_expression(&assign.rhs);
            }
            Expression::Call(call) => {
                match call {
                    Call::Function(func_call) => {
                        for arg in func_call.argument_list.arguments.iter() {
                            self.analyze_expression(arg.value());
                        }
                    }
                    Call::Method(method_call) => {
                        self.analyze_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.analyze_expression(arg.value());
                        }
                    }
                    Call::NullSafeMethod(method_call) => {
                        self.analyze_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.analyze_expression(arg.value());
                        }
                    }
                    Call::StaticMethod(static_call) => {
                        for arg in static_call.argument_list.arguments.iter() {
                            self.analyze_expression(arg.value());
                        }
                    }
                }
            }
            Expression::Array(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.key);
                            self.analyze_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.analyze_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::LegacyArray(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.key);
                            self.analyze_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.analyze_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::Parenthesized(p) => {
                self.analyze_expression(&p.expression);
            }
            Expression::ArrayAccess(arr) => {
                self.analyze_expression(&arr.array);
                self.analyze_expression(&arr.index);
            }
            _ => {}
        }
    }

    fn analyze_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.analyze_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition);
                    self.analyze_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.analyze_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition);
                    for stmt in else_if.statements.iter() {
                        self.analyze_statement(stmt);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for stmt in else_clause.statements.iter() {
                        self.analyze_statement(stmt);
                    }
                }
            }
        }
    }

    fn analyze_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => self.analyze_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
        }
    }

    fn analyze_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => self.analyze_statement(stmt),
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
        }
    }

    fn analyze_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => self.analyze_statement(stmt),
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_always_false_boolean_check_level() {
        let check = AlwaysFalseBooleanCheck;
        assert_eq!(check.level(), 4);
    }
}
