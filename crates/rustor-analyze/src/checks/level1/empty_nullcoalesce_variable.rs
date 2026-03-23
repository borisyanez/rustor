//! Checks for undefined variables in empty() and ?? expressions (Level 1)
//!
//! - empty.variable: variable used in empty() that is never defined in scope
//! - nullCoalesce.variable: variable on left side of ?? that is never defined in scope

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashSet;
use std::path::PathBuf;

const SUPERGLOBALS: &[&str] = &[
    "$_GET", "$_POST", "$_REQUEST", "$_SERVER", "$_SESSION", "$_COOKIE",
    "$_FILES", "$_ENV", "$GLOBALS", "$this",
];

/// Detects variables used in empty($var) or $var ?? expr that are never defined anywhere in scope
pub struct EmptyNullCoalesceVariableCheck;

impl Check for EmptyNullCoalesceVariableCheck {
    fn id(&self) -> &'static str {
        "empty.variable"
    }

    fn description(&self) -> &'static str {
        "Detects variables used in empty() or ?? that are never defined in scope"
    }

    fn level(&self) -> u8 {
        1
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = EmptyCoalesceAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };
        // Process each top-level statement; global scope vars accumulate in parent_defined
        let mut global_defined: HashSet<String> = HashSet::new();
        for sg in SUPERGLOBALS {
            global_defined.insert(sg.to_string());
        }
        // Collect all top-level assignments first (for global scope)
        for stmt in program.statements.iter() {
            analyzer.collect_defined_in_stmt(stmt, &mut global_defined, false);
        }
        for stmt in program.statements.iter() {
            analyzer.analyze_stmt(stmt, &global_defined);
        }
        analyzer.issues
    }
}

struct EmptyCoalesceAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

impl<'s> EmptyCoalesceAnalyzer<'s> {
    fn span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn get_line_col(&self, offset: usize) -> (usize, usize) {
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

    fn is_superglobal(&self, name: &str) -> bool {
        SUPERGLOBALS.contains(&name)
    }

    /// Collect variable definitions from a statement into `defined`.
    /// `recurse_into_functions`: if false, skip nested function/class bodies.
    fn collect_defined_in_stmt<'a>(
        &self,
        stmt: &Statement<'a>,
        defined: &mut HashSet<String>,
        recurse_into_functions: bool,
    ) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.collect_defined_in_expr(&expr_stmt.expression, defined, recurse_into_functions);
            }
            Statement::If(if_stmt) => {
                self.collect_defined_in_expr(&if_stmt.condition, defined, recurse_into_functions);
                match &if_stmt.body {
                    IfBody::Statement(body) => {
                        self.collect_defined_in_stmt(body.statement, defined, recurse_into_functions);
                        for elseif in body.else_if_clauses.iter() {
                            self.collect_defined_in_expr(&elseif.condition, defined, recurse_into_functions);
                            self.collect_defined_in_stmt(elseif.statement, defined, recurse_into_functions);
                        }
                        if let Some(else_clause) = &body.else_clause {
                            self.collect_defined_in_stmt(else_clause.statement, defined, recurse_into_functions);
                        }
                    }
                    IfBody::ColonDelimited(body) => {
                        for s in body.statements.iter() {
                            self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                        }
                        for elseif in body.else_if_clauses.iter() {
                            self.collect_defined_in_expr(&elseif.condition, defined, recurse_into_functions);
                            for s in elseif.statements.iter() {
                                self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                            }
                        }
                        if let Some(else_clause) = &body.else_clause {
                            for s in else_clause.statements.iter() {
                                self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                            }
                        }
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.collect_defined_in_expr(&while_stmt.condition, defined, recurse_into_functions);
                match &while_stmt.body {
                    WhileBody::Statement(s) => {
                        self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                    }
                    WhileBody::ColonDelimited(b) => {
                        for s in b.statements.iter() {
                            self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                        }
                    }
                }
            }
            Statement::For(for_stmt) => {
                for init in for_stmt.initializations.iter() {
                    self.collect_defined_in_expr(init, defined, recurse_into_functions);
                }
                match &for_stmt.body {
                    ForBody::Statement(s) => {
                        self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                    }
                    ForBody::ColonDelimited(b) => {
                        for s in b.statements.iter() {
                            self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                        }
                    }
                }
            }
            Statement::Foreach(foreach) => {
                // foreach loop variable(s) are defined
                if let ForeachTarget::KeyValue(kv) = &foreach.target {
                    if let Expression::Variable(Variable::Direct(v)) = &kv.key {
                        defined.insert(self.span_text(&v.span).to_string());
                    }
                    if let Expression::Variable(Variable::Direct(v)) = &kv.value {
                        defined.insert(self.span_text(&v.span).to_string());
                    }
                } else if let ForeachTarget::Value(val) = &foreach.target {
                    if let Expression::Variable(Variable::Direct(v)) = &val.value {
                        defined.insert(self.span_text(&v.span).to_string());
                    }
                }
                match &foreach.body {
                    ForeachBody::Statement(s) => {
                        self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                    }
                    ForeachBody::ColonDelimited(b) => {
                        for s in b.statements.iter() {
                            self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                        }
                    }
                }
            }
            Statement::Try(try_stmt) => {
                for s in try_stmt.block.statements.iter() {
                    self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                }
                for catch in try_stmt.catch_clauses.iter() {
                    if let Some(var) = &catch.variable {
                        let name = self.span_text(&var.span());
                        defined.insert(name.to_string());
                    }
                    for s in catch.block.statements.iter() {
                        self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    for s in finally.block.statements.iter() {
                        self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                    }
                }
            }
            Statement::Block(block) => {
                for s in block.statements.iter() {
                    self.collect_defined_in_stmt(s, defined, recurse_into_functions);
                }
            }
            Statement::Global(global) => {
                for var in global.variables.iter() {
                    let name = self.span_text(&var.span());
                    defined.insert(name.to_string());
                }
            }
            Statement::Static(static_stmt) => {
                for item in static_stmt.items.iter() {
                    let name = self.span_text(&item.variable().span());
                    defined.insert(name.to_string());
                }
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.collect_defined_in_expr(value, defined, recurse_into_functions);
                }
            }
            Statement::Echo(echo) => {
                for value in echo.values.iter() {
                    self.collect_defined_in_expr(value, defined, recurse_into_functions);
                }
            }
            // Don't recurse into nested function/class definitions (they have their own scope)
            Statement::Function(_) | Statement::Class(_) => {}
            _ => {}
        }
    }

    fn collect_defined_in_expr<'a>(
        &self,
        expr: &Expression<'a>,
        defined: &mut HashSet<String>,
        recurse_into_functions: bool,
    ) {
        match expr {
            Expression::Assignment(assign) => {
                // LHS variable gets defined
                if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                    defined.insert(self.span_text(&var.span).to_string());
                }
                self.collect_defined_in_expr(&assign.rhs, defined, recurse_into_functions);
            }
            Expression::Binary(binary) => {
                self.collect_defined_in_expr(&binary.lhs, defined, recurse_into_functions);
                self.collect_defined_in_expr(&binary.rhs, defined, recurse_into_functions);
            }
            Expression::Conditional(cond) => {
                self.collect_defined_in_expr(&cond.condition, defined, recurse_into_functions);
                if let Some(then) = &cond.then {
                    self.collect_defined_in_expr(then, defined, recurse_into_functions);
                }
                self.collect_defined_in_expr(&cond.r#else, defined, recurse_into_functions);
            }
            Expression::Call(call) => match call {
                Call::Function(fc) => {
                    for arg in fc.argument_list.arguments.iter() {
                        self.collect_defined_in_expr(arg.value(), defined, recurse_into_functions);
                    }
                }
                Call::Method(mc) => {
                    self.collect_defined_in_expr(&mc.object, defined, recurse_into_functions);
                    for arg in mc.argument_list.arguments.iter() {
                        self.collect_defined_in_expr(arg.value(), defined, recurse_into_functions);
                    }
                }
                Call::NullSafeMethod(mc) => {
                    self.collect_defined_in_expr(&mc.object, defined, recurse_into_functions);
                    for arg in mc.argument_list.arguments.iter() {
                        self.collect_defined_in_expr(arg.value(), defined, recurse_into_functions);
                    }
                }
                Call::StaticMethod(sc) => {
                    for arg in sc.argument_list.arguments.iter() {
                        self.collect_defined_in_expr(arg.value(), defined, recurse_into_functions);
                    }
                }
            },
            Expression::Parenthesized(p) => {
                self.collect_defined_in_expr(&p.expression, defined, recurse_into_functions);
            }
            // Don't recurse into closures/arrow functions for outer scope collection
            Expression::Closure(_) | Expression::ArrowFunction(_) => {}
            _ => {}
        }
    }

    /// Analyze a statement for empty($var) / $var ?? issues using the given defined set.
    fn analyze_stmt<'a>(&mut self, stmt: &Statement<'a>, defined: &HashSet<String>) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.analyze_expr(&expr_stmt.expression, defined);
            }
            Statement::If(if_stmt) => {
                self.analyze_expr(&if_stmt.condition, defined);
                match &if_stmt.body {
                    IfBody::Statement(body) => {
                        self.analyze_stmt(body.statement, defined);
                        for elseif in body.else_if_clauses.iter() {
                            self.analyze_expr(&elseif.condition, defined);
                            self.analyze_stmt(elseif.statement, defined);
                        }
                        if let Some(else_clause) = &body.else_clause {
                            self.analyze_stmt(else_clause.statement, defined);
                        }
                    }
                    IfBody::ColonDelimited(body) => {
                        for s in body.statements.iter() {
                            self.analyze_stmt(s, defined);
                        }
                        for elseif in body.else_if_clauses.iter() {
                            self.analyze_expr(&elseif.condition, defined);
                            for s in elseif.statements.iter() {
                                self.analyze_stmt(s, defined);
                            }
                        }
                        if let Some(else_clause) = &body.else_clause {
                            for s in else_clause.statements.iter() {
                                self.analyze_stmt(s, defined);
                            }
                        }
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.analyze_expr(&while_stmt.condition, defined);
                match &while_stmt.body {
                    WhileBody::Statement(s) => self.analyze_stmt(s, defined),
                    WhileBody::ColonDelimited(b) => {
                        for s in b.statements.iter() {
                            self.analyze_stmt(s, defined);
                        }
                    }
                }
            }
            Statement::For(for_stmt) => {
                for init in for_stmt.initializations.iter() {
                    self.analyze_expr(init, defined);
                }
                for cond in for_stmt.conditions.iter() {
                    self.analyze_expr(cond, defined);
                }
                for inc in for_stmt.increments.iter() {
                    self.analyze_expr(inc, defined);
                }
                match &for_stmt.body {
                    ForBody::Statement(s) => self.analyze_stmt(s, defined),
                    ForBody::ColonDelimited(b) => {
                        for s in b.statements.iter() {
                            self.analyze_stmt(s, defined);
                        }
                    }
                }
            }
            Statement::Foreach(foreach) => {
                self.analyze_expr(&foreach.expression, defined);
                match &foreach.body {
                    ForeachBody::Statement(s) => self.analyze_stmt(s, defined),
                    ForeachBody::ColonDelimited(b) => {
                        for s in b.statements.iter() {
                            self.analyze_stmt(s, defined);
                        }
                    }
                }
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.analyze_expr(value, defined);
                }
            }
            Statement::Echo(echo) => {
                for value in echo.values.iter() {
                    self.analyze_expr(value, defined);
                }
            }
            Statement::Block(block) => {
                for s in block.statements.iter() {
                    self.analyze_stmt(s, defined);
                }
            }
            Statement::Try(try_stmt) => {
                for s in try_stmt.block.statements.iter() {
                    self.analyze_stmt(s, defined);
                }
                for catch in try_stmt.catch_clauses.iter() {
                    for s in catch.block.statements.iter() {
                        self.analyze_stmt(s, defined);
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    for s in finally.block.statements.iter() {
                        self.analyze_stmt(s, defined);
                    }
                }
            }
            // Nested function/class defs: build their own scope and recurse
            Statement::Function(func) => {
                let mut func_defined: HashSet<String> = HashSet::new();
                for sg in SUPERGLOBALS {
                    func_defined.insert(sg.to_string());
                }
                for param in func.parameter_list.parameters.iter() {
                    let name = self.span_text(&param.variable.span());
                    func_defined.insert(name.to_string());
                }
                // Collect all assignments in the body
                for s in func.body.statements.iter() {
                    self.collect_defined_in_stmt(s, &mut func_defined, false);
                }
                for s in func.body.statements.iter() {
                    self.analyze_stmt(s, &func_defined);
                }
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            let mut method_defined: HashSet<String> = HashSet::new();
                            for sg in SUPERGLOBALS {
                                method_defined.insert(sg.to_string());
                            }
                            method_defined.insert("$this".to_string());
                            for param in method.parameter_list.parameters.iter() {
                                let name = self.span_text(&param.variable.span());
                                method_defined.insert(name.to_string());
                            }
                            for s in body.statements.iter() {
                                self.collect_defined_in_stmt(s, &mut method_defined, false);
                            }
                            for s in body.statements.iter() {
                                self.analyze_stmt(s, &method_defined);
                            }
                        }
                    }
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for s in body.statements.iter() {
                        self.analyze_stmt(s, defined);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for s in body.statements.iter() {
                        self.analyze_stmt(s, defined);
                    }
                }
            },
            _ => {}
        }
    }

    fn analyze_expr<'a>(&mut self, expr: &Expression<'a>, defined: &HashSet<String>) {
        match expr {
            // empty($var) - check if $var is never defined
            Expression::Construct(Construct::Empty(empty)) => {
                if let Expression::Variable(Variable::Direct(var)) = empty.value {
                    let var_name = self.span_text(&var.span).to_string();
                    if !self.is_superglobal(&var_name) && !defined.contains(&var_name) {
                        let (line, col) =
                            self.get_line_col(empty.value.span().start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "empty.variable",
                                format!("Variable {} in empty() is never defined.", var_name),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("empty.variable"),
                        );
                    }
                }
                // Recurse into the value expression regardless
                self.analyze_expr(empty.value, defined);
            }
            // $var ?? expr - check if $var (LHS) is never defined
            Expression::Binary(binary)
                if matches!(binary.operator, BinaryOperator::NullCoalesce(_)) =>
            {
                if let Expression::Variable(Variable::Direct(var)) = &*binary.lhs {
                    let var_name = self.span_text(&var.span).to_string();
                    if !self.is_superglobal(&var_name) && !defined.contains(&var_name) {
                        let (line, col) =
                            self.get_line_col(binary.lhs.span().start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "nullCoalesce.variable",
                                format!(
                                    "Variable {} on left side of ?? is never defined.",
                                    var_name
                                ),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("nullCoalesce.variable"),
                        );
                    }
                }
                // Still scan both sides for nested expressions
                self.analyze_expr(&binary.lhs, defined);
                self.analyze_expr(&binary.rhs, defined);
            }
            Expression::Binary(binary) => {
                self.analyze_expr(&binary.lhs, defined);
                self.analyze_expr(&binary.rhs, defined);
            }
            Expression::UnaryPrefix(unary) => {
                self.analyze_expr(&unary.operand, defined);
            }
            Expression::UnaryPostfix(postfix) => {
                self.analyze_expr(&postfix.operand, defined);
            }
            Expression::Conditional(cond) => {
                self.analyze_expr(&cond.condition, defined);
                if let Some(then) = &cond.then {
                    self.analyze_expr(then, defined);
                }
                self.analyze_expr(&cond.r#else, defined);
            }
            Expression::Assignment(assign) => {
                self.analyze_expr(&assign.rhs, defined);
            }
            Expression::Call(call) => match call {
                Call::Function(fc) => {
                    for arg in fc.argument_list.arguments.iter() {
                        self.analyze_expr(arg.value(), defined);
                    }
                }
                Call::Method(mc) => {
                    self.analyze_expr(&mc.object, defined);
                    for arg in mc.argument_list.arguments.iter() {
                        self.analyze_expr(arg.value(), defined);
                    }
                }
                Call::NullSafeMethod(mc) => {
                    self.analyze_expr(&mc.object, defined);
                    for arg in mc.argument_list.arguments.iter() {
                        self.analyze_expr(arg.value(), defined);
                    }
                }
                Call::StaticMethod(sc) => {
                    for arg in sc.argument_list.arguments.iter() {
                        self.analyze_expr(arg.value(), defined);
                    }
                }
            },
            Expression::Array(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expr(&kv.key, defined);
                            self.analyze_expr(&kv.value, defined);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expr(&val.value, defined);
                        }
                        ArrayElement::Variadic(v) => {
                            self.analyze_expr(&v.value, defined);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::LegacyArray(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expr(&kv.key, defined);
                            self.analyze_expr(&kv.value, defined);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expr(&val.value, defined);
                        }
                        ArrayElement::Variadic(v) => {
                            self.analyze_expr(&v.value, defined);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::Parenthesized(p) => {
                self.analyze_expr(&p.expression, defined);
            }
            Expression::ArrayAccess(arr) => {
                self.analyze_expr(&arr.array, defined);
                self.analyze_expr(&arr.index, defined);
            }
            Expression::Closure(closure) => {
                // Build new defined set for closure scope
                let mut closure_defined: HashSet<String> = HashSet::new();
                for sg in SUPERGLOBALS {
                    closure_defined.insert(sg.to_string());
                }
                for param in closure.parameter_list.parameters.iter() {
                    let name = self.span_text(&param.variable.span());
                    closure_defined.insert(name.to_string());
                }
                // `use` clause variables
                if let Some(use_clause) = &closure.use_clause {
                    for var in use_clause.variables.iter() {
                        let name = self.span_text(&var.variable.span());
                        closure_defined.insert(name.to_string());
                    }
                }
                // Collect assignments in closure body
                for s in closure.body.statements.iter() {
                    self.collect_defined_in_stmt(s, &mut closure_defined, false);
                }
                for s in closure.body.statements.iter() {
                    self.analyze_stmt(s, &closure_defined);
                }
            }
            Expression::ArrowFunction(arrow) => {
                // Arrow functions capture outer scope
                let mut arrow_defined = defined.clone();
                for param in arrow.parameter_list.parameters.iter() {
                    let name = self.span_text(&param.variable.span());
                    arrow_defined.insert(name.to_string());
                }
                self.analyze_expr(arrow.expression, &arrow_defined);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_level() {
        let check = EmptyNullCoalesceVariableCheck;
        assert_eq!(check.level(), 1);
        assert_eq!(check.id(), "empty.variable");
    }
}
