//! Check for missing return statements (Level 0)
//!
//! PHPStan checks for missing return statements at level 0 when a function/method
//! declares a non-void return type but has no return statement.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::path::PathBuf;

/// Checks for missing return statements in functions/methods with return types
pub struct MissingReturnCheck;

impl Check for MissingReturnCheck {
    fn id(&self) -> &'static str {
        "return.missing"
    }

    fn description(&self) -> &'static str {
        "Detects functions with declared return types but no return statement"
    }

    fn level(&self) -> u8 {
        0 // PHPStan checks this at level 0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = MissingReturnAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };

        analyzer.analyze_program(program);
        analyzer.issues
    }
}

struct MissingReturnAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

impl<'s> MissingReturnAnalyzer<'s> {
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

    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                self.check_function(func);
            }
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        self.check_method(method, &class_name);
                    }
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
            _ => {}
        }
    }

    fn check_function<'a>(&mut self, func: &Function<'a>) {
        let return_type = self.extract_return_type(&func.return_type_hint);

        // Skip void, never, and functions without return type
        if return_type.is_none() {
            return;
        }
        let return_type_str = return_type.as_deref().unwrap();
        let rt_lower = return_type_str.to_lowercase();
        if rt_lower == "void" || rt_lower == "never" {
            return;
        }

        // Check if function body has any return statement
        let mut finder = ReturnFinder::new();
        finder.check_block(&func.body, self.source);

        if !finder.found {
            let func_name = self.get_span_text(&func.name.span);
            let (line, col) = self.get_line_col(func.name.span.start.offset as usize);
            self.issues.push(
                Issue::error(
                    "return.missing",
                    format!(
                        "Function {}() should return {} but return statement is missing.",
                        func_name, return_type_str
                    ),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("return.missing"),
            );
        }
    }

    fn check_method<'a>(&mut self, method: &Method<'a>, class_name: &str) {
        // Skip abstract methods (no body)
        let body = match &method.body {
            MethodBody::Concrete(body) => body,
            MethodBody::Abstract(_) => return,
        };

        let return_type = self.extract_return_type(&method.return_type_hint);

        // Skip void, never, and methods without return type
        if return_type.is_none() {
            return;
        }
        let return_type_str = return_type.as_deref().unwrap();
        let rt_lower = return_type_str.to_lowercase();
        if rt_lower == "void" || rt_lower == "never" {
            return;
        }

        // Skip constructor/destructor
        let method_name = self.get_span_text(&method.name.span);
        if method_name.eq_ignore_ascii_case("__construct") || method_name.eq_ignore_ascii_case("__destruct") {
            return;
        }

        // Check if method body has any return statement
        let mut finder = ReturnFinder::new();
        finder.check_block(body, self.source);

        if !finder.found {
            let (line, col) = self.get_line_col(method.name.span.start.offset as usize);
            self.issues.push(
                Issue::error(
                    "return.missing",
                    format!(
                        "Method {}::{}() should return {} but return statement is missing.",
                        class_name, method_name, return_type_str
                    ),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("return.missing"),
            );
        }
    }

    fn extract_return_type(&self, hint: &Option<FunctionLikeReturnTypeHint<'_>>) -> Option<String> {
        hint.as_ref().map(|h| self.get_span_text(&h.hint.span()).to_string())
    }
}

/// Simple visitor to find any return statement
struct ReturnFinder {
    found: bool,
}

impl ReturnFinder {
    fn new() -> Self {
        Self { found: false }
    }

    fn check_block<'a>(&mut self, block: &Block<'a>, source: &str) {
        for stmt in block.statements.iter() {
            self.check_statement(stmt, source);
            if self.found {
                return;
            }
        }
    }

    fn check_statement<'a>(&mut self, stmt: &Statement<'a>, source: &str) {
        if self.found {
            return;
        }

        match stmt {
            Statement::Return(_) => {
                self.found = true;
            }
            Statement::Expression(expr_stmt) => {
                // Check if this is a yield expression (generator function)
                if self.expression_contains_yield(&expr_stmt.expression) {
                    self.found = true;
                }
            }
            Statement::Function(_) => {
                // Don't descend into nested functions
            }
            Statement::Block(block) => {
                self.check_block(block, source);
            }
            Statement::If(if_stmt) => {
                self.check_if_body(&if_stmt.body, source);
            }
            Statement::Try(try_stmt) => {
                self.check_block(&try_stmt.block, source);
                for catch in try_stmt.catch_clauses.iter() {
                    self.check_block(&catch.block, source);
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    self.check_block(&finally.block, source);
                }
            }
            Statement::Switch(switch) => {
                self.check_switch(switch, source);
            }
            Statement::While(w) => {
                self.check_while_body(&w.body, source);
            }
            Statement::DoWhile(_) => {
                // DoWhile body handling skipped for simplicity
            }
            Statement::For(f) => {
                self.check_for_body(&f.body, source);
            }
            Statement::Foreach(fe) => {
                self.check_foreach_body(&fe.body, source);
            }
            _ => {}
        }
    }

    /// Check if an expression contains yield (making it a generator)
    fn expression_contains_yield<'a>(&self, expr: &Expression<'a>) -> bool {
        match expr {
            Expression::Yield(_) => true,
            Expression::Binary(binary) => {
                self.expression_contains_yield(&binary.lhs) ||
                self.expression_contains_yield(&binary.rhs)
            }
            Expression::Conditional(cond) => {
                self.expression_contains_yield(&cond.condition) ||
                cond.then.as_ref().map_or(false, |t| self.expression_contains_yield(t)) ||
                self.expression_contains_yield(&cond.r#else)
            }
            Expression::Parenthesized(p) => self.expression_contains_yield(&p.expression),
            Expression::Assignment(assign) => self.expression_contains_yield(assign.rhs),
            _ => false,
        }
    }

    fn check_if_body<'a>(&mut self, body: &IfBody<'a>, source: &str) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.check_statement(stmt_body.statement, source);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.check_statement(else_if.statement, source);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.check_statement(else_clause.statement, source);
                }
            }
            IfBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt, source);
                }
                for else_if in block.else_if_clauses.iter() {
                    for stmt in else_if.statements.iter() {
                        self.check_statement(stmt, source);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for stmt in else_clause.statements.iter() {
                        self.check_statement(stmt, source);
                    }
                }
            }
        }
    }

    fn check_while_body<'a>(&mut self, body: &WhileBody<'a>, source: &str) {
        match body {
            WhileBody::Statement(stmt) => {
                self.check_statement(stmt, source);
            }
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt, source);
                }
            }
        }
    }

    fn check_for_body<'a>(&mut self, body: &ForBody<'a>, source: &str) {
        match body {
            ForBody::Statement(stmt) => {
                self.check_statement(stmt, source);
            }
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt, source);
                }
            }
        }
    }

    fn check_foreach_body<'a>(&mut self, body: &ForeachBody<'a>, source: &str) {
        match body {
            ForeachBody::Statement(stmt) => {
                self.check_statement(stmt, source);
            }
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt, source);
                }
            }
        }
    }

    fn check_switch<'a>(&mut self, switch: &Switch<'a>, source: &str) {
        match &switch.body {
            SwitchBody::BraceDelimited(body) => {
                for case in body.cases.iter() {
                    match case {
                        SwitchCase::Expression(c) => {
                            for stmt in c.statements.iter() {
                                self.check_statement(stmt, source);
                            }
                        }
                        SwitchCase::Default(d) => {
                            for stmt in d.statements.iter() {
                                self.check_statement(stmt, source);
                            }
                        }
                    }
                }
            }
            SwitchBody::ColonDelimited(body) => {
                for case in body.cases.iter() {
                    match case {
                        SwitchCase::Expression(c) => {
                            for stmt in c.statements.iter() {
                                self.check_statement(stmt, source);
                            }
                        }
                        SwitchCase::Default(d) => {
                            for stmt in d.statements.iter() {
                                self.check_statement(stmt, source);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_return_check_level() {
        let check = MissingReturnCheck;
        assert_eq!(check.level(), 0); // Should be level 0 like PHPStan
    }
}
