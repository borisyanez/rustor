//! Check for undefined variables

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct UndefinedVariableCheck;

impl Check for UndefinedVariableCheck {
    fn id(&self) -> &'static str {
        "undefined.variable"
    }

    fn description(&self) -> &'static str {
        "Detects use of undefined variables"
    }

    fn level(&self) -> u8 {
        1
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = VariableAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            scopes: vec![Scope::new()],
            issues: Vec::new(),
        };

        // Add superglobals to the global scope
        analyzer.define_superglobals();

        analyzer.analyze_program(program);
        analyzer.issues
    }
}

/// A scope containing defined variables
#[derive(Debug, Clone)]
struct Scope {
    defined: HashSet<String>,
    /// Whether this is a closure scope (closures don't inherit parent scope unless via `use`)
    is_closure: bool,
    /// Variables inherited via closure `use` clause
    inherited: HashSet<String>,
}

impl Scope {
    fn new() -> Self {
        Self {
            defined: HashSet::new(),
            is_closure: false,
            inherited: HashSet::new(),
        }
    }

    fn closure() -> Self {
        Self {
            defined: HashSet::new(),
            is_closure: true,
            inherited: HashSet::new(),
        }
    }

    fn define(&mut self, name: String) {
        self.defined.insert(name);
    }

    fn is_defined(&self, name: &str) -> bool {
        self.defined.contains(name) || self.inherited.contains(name)
    }
}

struct VariableAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    scopes: Vec<Scope>,
    issues: Vec<Issue>,
}

impl<'s> VariableAnalyzer<'s> {
    fn define_superglobals(&mut self) {
        let superglobals = [
            "$_GET", "$_POST", "$_REQUEST", "$_SERVER", "$_SESSION", "$_COOKIE",
            "$_FILES", "$_ENV", "$GLOBALS", "$this", "$argc", "$argv",
        ];
        for var in superglobals {
            self.current_scope_mut().define(var.to_string());
        }
    }

    fn current_scope(&self) -> &Scope {
        self.scopes.last().unwrap()
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn push_closure_scope(&mut self) {
        self.scopes.push(Scope::closure());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn is_defined(&self, name: &str) -> bool {
        // For closure scopes, only check the closure scope itself
        if self.current_scope().is_closure {
            return self.current_scope().is_defined(name);
        }

        // Check all scopes from current to global
        for scope in self.scopes.iter().rev() {
            if scope.is_defined(name) {
                return true;
            }
            // Stop at closure boundary unless checking inherited vars
            if scope.is_closure {
                break;
            }
        }
        false
    }

    fn define(&mut self, name: String) {
        self.current_scope_mut().define(name);
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
            Statement::Expression(expr_stmt) => {
                self.analyze_expression(&expr_stmt.expression, false);
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::If(if_stmt) => {
                self.analyze_expression(&if_stmt.condition, false);
                self.analyze_if_body(&if_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.analyze_expression(&foreach.expression, false);

                // Define the loop variables
                if let ForeachTarget::KeyValue(kv) = &foreach.target {
                    let key = self.get_var_name(&kv.key);
                    if let Some(name) = key {
                        self.define(name);
                    }
                    let value = self.get_var_name(&kv.value);
                    if let Some(name) = value {
                        self.define(name);
                    }
                } else if let ForeachTarget::Value(value) = &foreach.target {
                    let name = self.get_var_name(&value.value);
                    if let Some(n) = name {
                        self.define(n);
                    }
                }

                self.analyze_foreach_body(&foreach.body);
            }
            Statement::For(for_stmt) => {
                for expr in for_stmt.initializations.iter() {
                    self.analyze_expression(expr, false);
                }
                for expr in for_stmt.conditions.iter() {
                    self.analyze_expression(expr, false);
                }
                for expr in for_stmt.increments.iter() {
                    self.analyze_expression(expr, false);
                }
                self.analyze_for_body(&for_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.analyze_expression(&while_stmt.condition, false);
                self.analyze_while_body(&while_stmt.body);
            }
            Statement::DoWhile(do_while) => {
                self.analyze_statement(&do_while.statement);
                self.analyze_expression(&do_while.condition, false);
            }
            Statement::Try(try_stmt) => {
                for inner in try_stmt.block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for catch in try_stmt.catch_clauses.iter() {
                    // Define the exception variable
                    if let Some(var) = &catch.variable {
                        let span = var.span();
                        let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                        self.define(name.to_string());
                    }
                    for inner in catch.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    for inner in finally.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            }
            Statement::Switch(switch) => {
                self.analyze_expression(&switch.expression, false);
                self.analyze_switch_body(&switch.body);
            }
            Statement::Return(ret) => {
                if let Some(expr) = &ret.value {
                    self.analyze_expression(expr, false);
                }
            }
            Statement::Echo(echo) => {
                for expr in echo.values.iter() {
                    self.analyze_expression(expr, false);
                }
            }
            Statement::Function(func) => {
                self.push_scope();
                // Define parameters
                for param in func.parameter_list.parameters.iter() {
                    let span = param.variable.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
                }
                self.pop_scope();
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    self.analyze_class_member(member);
                }
            }
            Statement::Namespace(ns) => {
                match &ns.body {
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
                }
            }
            Statement::Global(global) => {
                // Global statement defines the variable in current scope
                for var in global.variables.iter() {
                    let span = var.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
            }
            Statement::Static(static_stmt) => {
                // Static statement defines the variable in current scope
                for item in static_stmt.items.iter() {
                    let span = item.variable().span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
            }
            _ => {}
        }
    }

    fn analyze_class_member<'a>(&mut self, member: &ClassLikeMember<'a>) {
        if let ClassLikeMember::Method(method) = member {
            if let MethodBody::Concrete(body) = &method.body {
                self.push_scope();
                // Define $this
                self.define("$this".to_string());
                // Define parameters
                for param in method.parameter_list.parameters.iter() {
                    let span = param.variable.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
                for inner in body.statements.iter() {
                    self.analyze_statement(inner);
                }
                self.pop_scope();
            }
        }
    }

    fn analyze_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.analyze_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition, false);
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
                    self.analyze_expression(&else_if.condition, false);
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

    fn analyze_switch_body<'a>(&mut self, body: &SwitchBody<'a>) {
        match body {
            SwitchBody::BraceDelimited(block) => {
                for case in block.cases.iter() {
                    for stmt in case.statements().iter() {
                        self.analyze_statement(stmt);
                    }
                }
            }
            SwitchBody::ColonDelimited(block) => {
                for case in block.cases.iter() {
                    for stmt in case.statements().iter() {
                        self.analyze_statement(stmt);
                    }
                }
            }
        }
    }

    fn get_var_name<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        if let Expression::Variable(var) = expr {
            let name = &self.source[var.span().start.offset as usize..var.span().end.offset as usize];
            return Some(name.to_string());
        }
        None
    }

    fn analyze_expression<'a>(&mut self, expr: &Expression<'a>, is_assignment_lhs: bool) {
        match expr {
            Expression::Variable(var) => {
                let name = &self.source[var.span().start.offset as usize..var.span().end.offset as usize];

                // If this is the left side of an assignment, it defines the variable
                if is_assignment_lhs {
                    self.define(name.to_string());
                } else {
                    // Otherwise, check if it's defined
                    if !self.is_defined(name) && !name.starts_with("$$") {
                        let (line, col) = self.get_line_col(var.span().start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "undefined.variable",
                                format!("Undefined variable {}", name),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("variable.undefined"),
                        );
                    }
                }
            }
            Expression::Assignment(assign) => {
                // First analyze RHS (before LHS is defined)
                self.analyze_expression(&assign.rhs, false);
                // Then analyze LHS as assignment target
                self.analyze_expression(&assign.lhs, true);
            }
            Expression::Closure(closure) => {
                self.push_closure_scope();

                // Add parameters to closure scope
                for param in closure.parameter_list.parameters.iter() {
                    let span = param.variable.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }

                // Add `use` variables to closure scope
                if let Some(use_clause) = &closure.use_clause {
                    for var in use_clause.variables.iter() {
                        let span = var.variable.span();
                        let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                        self.current_scope_mut().inherited.insert(name.to_string());
                    }
                }

                // Analyze closure body
                for stmt in closure.body.statements.iter() {
                    self.analyze_statement(stmt);
                }

                self.pop_scope();
            }
            Expression::ArrowFunction(arrow) => {
                // Arrow functions inherit parent scope
                // Add parameters
                for param in arrow.parameter_list.parameters.iter() {
                    let span = param.variable.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.define(name.to_string());
                }
                // Analyze expression
                self.analyze_expression(arrow.expression, false);
            }
            Expression::Call(Call::Function(call)) => {
                for arg in call.argument_list.arguments.iter() {
                    self.analyze_expression(arg.value(), false);
                }
            }
            Expression::Call(Call::Method(call)) => {
                self.analyze_expression(&call.object, false);
                for arg in call.argument_list.arguments.iter() {
                    self.analyze_expression(arg.value(), false);
                }
            }
            Expression::Call(Call::StaticMethod(call)) => {
                for arg in call.argument_list.arguments.iter() {
                    self.analyze_expression(arg.value(), false);
                }
            }
            Expression::Binary(binary) => {
                self.analyze_expression(&binary.lhs, false);
                self.analyze_expression(&binary.rhs, false);
            }
            Expression::Conditional(ternary) => {
                self.analyze_expression(&ternary.condition, false);
                if let Some(then) = &ternary.then {
                    self.analyze_expression(then, false);
                }
                self.analyze_expression(&ternary.r#else, false);
            }
            Expression::Parenthesized(paren) => {
                self.analyze_expression(&paren.expression, false);
            }
            Expression::UnaryPrefix(unary) => {
                self.analyze_expression(&unary.operand, false);
            }
            Expression::UnaryPostfix(unary) => {
                self.analyze_expression(&unary.operand, false);
            }
            Expression::ArrayAccess(access) => {
                self.analyze_expression(&access.array, is_assignment_lhs);
                self.analyze_expression(&access.index, false);
            }
            Expression::Access(Access::Property(access)) => {
                self.analyze_expression(&access.object, false);
            }
            Expression::Access(Access::NullSafeProperty(access)) => {
                self.analyze_expression(&access.object, false);
            }
            Expression::Array(arr) => {
                for elem in arr.elements.iter() {
                    match elem {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.key, false);
                            self.analyze_expression(&kv.value, false);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value, false);
                        }
                        ArrayElement::Variadic(var) => {
                            self.analyze_expression(&var.value, false);
                        }
                        _ => {}
                    }
                }
            }
            Expression::LegacyArray(arr) => {
                for elem in arr.elements.iter() {
                    match elem {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.key, false);
                            self.analyze_expression(&kv.value, false);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value, false);
                        }
                        ArrayElement::Variadic(var) => {
                            self.analyze_expression(&var.value, false);
                        }
                        _ => {}
                    }
                }
            }
            Expression::Instantiation(instantiate) => {
                if let Some(arg_list) = &instantiate.argument_list {
                    for arg in arg_list.arguments.iter() {
                        self.analyze_expression(arg.value(), false);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_operations() {
        let mut scope = Scope::new();
        scope.define("$foo".to_string());
        assert!(scope.is_defined("$foo"));
        assert!(!scope.is_defined("$bar"));
    }
}
