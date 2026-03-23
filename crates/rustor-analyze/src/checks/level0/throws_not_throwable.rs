//! Check for throwing non-Throwable values
//!
//! - `throws.notThrowable`: Only Throwable can be thrown

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::path::PathBuf;

/// Detects `throw` expressions that throw non-Throwable values (literals, etc.)
pub struct ThrowsNotThrowableCheck;

impl Check for ThrowsNotThrowableCheck {
    fn id(&self) -> &'static str {
        "throws.notThrowable"
    }

    fn description(&self) -> &'static str {
        "Detects throw expressions that throw non-Throwable values"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = ThrowsAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };
        analyzer.analyze_program(program);
        analyzer.issues
    }
}

struct ThrowsAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

impl<'s> ThrowsAnalyzer<'s> {
    fn line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1usize;
        let mut col = 1usize;
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
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            for inner in body.statements.iter() {
                                self.analyze_statement(inner);
                            }
                        }
                    }
                }
            }
            Statement::Function(func) => {
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
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
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::If(if_stmt) => {
                self.analyze_if_body(&if_stmt.body);
            }
            Statement::While(w) => {
                self.analyze_while_body(&w.body);
            }
            Statement::For(f) => {
                self.analyze_for_body(&f.body);
            }
            Statement::Foreach(fe) => {
                self.analyze_foreach_body(&fe.body);
            }
            Statement::Try(t) => {
                for inner in t.block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for catch in t.catch_clauses.iter() {
                    for inner in catch.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                if let Some(finally) = &t.finally_clause {
                    for inner in finally.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            }
            Statement::Switch(sw) => {
                self.analyze_switch(sw);
            }
            _ => {}
        }
    }

    fn analyze_expression<'a>(&mut self, expr: &Expression<'a>) {
        if let Expression::Throw(throw_expr) = expr {
            self.check_throw(throw_expr);
        }
    }

    fn analyze_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.analyze_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.analyze_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.analyze_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for else_if in block.else_if_clauses.iter() {
                    for inner in else_if.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            }
        }
    }

    fn analyze_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_switch<'a>(&mut self, switch: &Switch<'a>) {
        match &switch.body {
            SwitchBody::BraceDelimited(b) => {
                for case in b.cases.iter() {
                    let stmts: Vec<_> = match case {
                        SwitchCase::Expression(c) => c.statements.iter().collect(),
                        SwitchCase::Default(d) => d.statements.iter().collect(),
                    };
                    for stmt in stmts {
                        self.analyze_statement(stmt);
                    }
                }
            }
            SwitchBody::ColonDelimited(b) => {
                for case in b.cases.iter() {
                    let stmts: Vec<_> = match case {
                        SwitchCase::Expression(c) => c.statements.iter().collect(),
                        SwitchCase::Default(d) => d.statements.iter().collect(),
                    };
                    for stmt in stmts {
                        self.analyze_statement(stmt);
                    }
                }
            }
        }
    }

    fn check_throw<'a>(&mut self, throw_expr: &Throw<'a>) {
        let expr = &throw_expr.exception;

        // Check if the expression is a literal
        if let Expression::Literal(_) = expr {
            let (line, col) = self.line_col(throw_expr.span().start.offset as usize);
            self.issues.push(
                Issue::error(
                    "throws.notThrowable",
                    "Only Throwable can be thrown, literal given.".to_string(),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("throws.notThrowable"),
            );
        }
        // Variables and `new ClassName()` are assumed to be Throwable (can't verify statically without type info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_throws_not_throwable_check_level() {
        assert_eq!(ThrowsNotThrowableCheck.level(), 0);
        assert_eq!(ThrowsNotThrowableCheck.id(), "throws.notThrowable");
    }
}
