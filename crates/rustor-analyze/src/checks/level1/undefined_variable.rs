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
        0
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
    /// Variables that are definitely defined in all code paths
    defined: HashSet<String>,
    /// Variables that are possibly defined (in some but not all code paths)
    possibly_defined: HashSet<String>,
    /// Whether this is a closure scope (closures don't inherit parent scope unless via `use`)
    is_closure: bool,
    /// Variables inherited via closure `use` clause
    inherited: HashSet<String>,
    /// Whether $this is available in this scope (inside a class method)
    has_this: bool,
}

impl Scope {
    fn new() -> Self {
        Self {
            defined: HashSet::new(),
            possibly_defined: HashSet::new(),
            is_closure: false,
            inherited: HashSet::new(),
            has_this: false,
        }
    }

    fn closure() -> Self {
        Self {
            defined: HashSet::new(),
            possibly_defined: HashSet::new(),
            is_closure: true,
            inherited: HashSet::new(),
            has_this: false,
        }
    }

    fn define(&mut self, name: String) {
        // If we define a variable, it moves from possibly_defined to defined
        self.possibly_defined.remove(&name);
        self.defined.insert(name);
    }

    fn define_possibly(&mut self, name: String) {
        // Only add to possibly_defined if not already definitely defined
        if !self.defined.contains(&name) {
            self.possibly_defined.insert(name);
        }
    }

    fn is_defined(&self, name: &str) -> bool {
        self.defined.contains(name) || self.inherited.contains(name)
    }

    fn is_possibly_defined(&self, name: &str) -> bool {
        self.possibly_defined.contains(name)
    }

    /// Get snapshot of currently defined variables (for branch analysis)
    fn snapshot(&self) -> HashSet<String> {
        self.defined.clone()
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
            "$_FILES", "$_ENV", "$GLOBALS", "$argc", "$argv",
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

    fn push_method_scope(&mut self) {
        let mut scope = Scope::new();
        scope.has_this = true;
        scope.define("$this".to_string());
        self.scopes.push(scope);
    }

    fn push_closure_scope(&mut self) {
        let mut scope = Scope::closure();
        // In PHP 5.4+, closures automatically bind $this from enclosing class method
        if self.has_this_in_scope() {
            scope.has_this = true;
            scope.define("$this".to_string());
        }
        self.scopes.push(scope);
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Check if $this is available in the current scope chain
    fn has_this_in_scope(&self) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.has_this {
                return true;
            }
        }
        false
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

    fn is_possibly_defined(&self, name: &str) -> bool {
        // For closure scopes, only check the closure scope itself
        if self.current_scope().is_closure {
            return self.current_scope().is_possibly_defined(name);
        }

        // Check all scopes from current to global
        for scope in self.scopes.iter().rev() {
            if scope.is_possibly_defined(name) {
                return true;
            }
            // Stop at closure boundary
            if scope.is_closure {
                break;
            }
        }
        false
    }

    fn define(&mut self, name: String) {
        self.current_scope_mut().define(name);
    }

    fn define_possibly(&mut self, name: String) {
        self.current_scope_mut().define_possibly(name);
    }

    /// Merge branch results: variables defined in all branches become definitely defined,
    /// variables defined in some branches become possibly defined.
    fn merge_branches(&mut self, before_snapshot: &HashSet<String>, branch_snapshots: Vec<HashSet<String>>) {
        if branch_snapshots.is_empty() {
            return;
        }

        // Find variables that were newly defined in each branch
        let mut branch_new_vars: Vec<HashSet<String>> = branch_snapshots
            .iter()
            .map(|snap| snap.difference(before_snapshot).cloned().collect())
            .collect();

        // Variables defined in ALL branches become definitely defined
        if !branch_new_vars.is_empty() {
            let mut intersection: HashSet<String> = branch_new_vars[0].clone();
            for branch in branch_new_vars.iter().skip(1) {
                intersection = intersection.intersection(branch).cloned().collect();
            }
            for var in intersection {
                self.define(var);
            }
        }

        // Variables defined in SOME (but not all) branches become possibly defined
        let all_new_vars: HashSet<String> = branch_new_vars.drain(..).flatten().collect();
        for var in all_new_vars {
            if !self.is_defined(&var) {
                self.define_possibly(var);
            }
        }
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
                // Check if there's an else clause
                let has_else = match &if_stmt.body {
                    IfBody::Statement(stmt_body) => stmt_body.else_clause.is_some(),
                    IfBody::ColonDelimited(block) => block.else_clause.is_some(),
                };
                self.analyze_if_body(&if_stmt.body, has_else);
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
                // Use push_method_scope which sets has_this and defines $this
                self.push_method_scope();
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

    fn analyze_if_body<'a>(&mut self, body: &IfBody<'a>, has_else: bool) {
        let before_snapshot = self.current_scope().snapshot();
        let mut branch_snapshots = Vec::new();

        match body {
            IfBody::Statement(stmt_body) => {
                // Analyze 'if' branch
                self.analyze_statement(stmt_body.statement);
                branch_snapshots.push(self.current_scope().snapshot());

                // Reset to before state for each subsequent branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'elseif' branches
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition, false);
                    self.analyze_statement(else_if.statement);
                    branch_snapshots.push(self.current_scope().snapshot());
                    self.current_scope_mut().defined = before_snapshot.clone();
                }

                // Analyze 'else' branch
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.analyze_statement(else_clause.statement);
                    branch_snapshots.push(self.current_scope().snapshot());
                }
            }
            IfBody::ColonDelimited(block) => {
                // Analyze 'if' branch
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
                branch_snapshots.push(self.current_scope().snapshot());

                // Reset to before state for each subsequent branch
                self.current_scope_mut().defined = before_snapshot.clone();

                // Analyze 'elseif' branches
                for else_if in block.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition, false);
                    for inner in else_if.statements.iter() {
                        self.analyze_statement(inner);
                    }
                    branch_snapshots.push(self.current_scope().snapshot());
                    self.current_scope_mut().defined = before_snapshot.clone();
                }

                // Analyze 'else' branch
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        self.analyze_statement(inner);
                    }
                    branch_snapshots.push(self.current_scope().snapshot());
                }
            }
        }

        // Reset to original state before merging
        self.current_scope_mut().defined = before_snapshot.clone();

        // Merge branch results
        // If there's no else clause, we need to consider the "fall-through" path
        // where the if condition was false and no branch was taken
        if !has_else {
            // Add the "no branch taken" snapshot (same as before_snapshot)
            branch_snapshots.push(before_snapshot.clone());
        }

        self.merge_branches(&before_snapshot, branch_snapshots);
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
        let before_snapshot = self.current_scope().snapshot();
        let mut branch_snapshots = Vec::new();

        match body {
            SwitchBody::BraceDelimited(block) => {
                for case in block.cases.iter() {
                    // Analyze this case as a separate branch
                    for stmt in case.statements().iter() {
                        self.analyze_statement(stmt);
                    }

                    // Save the state after this case
                    branch_snapshots.push(self.current_scope().snapshot());

                    // Reset to before state for next case
                    // (PHP switch has fallthrough, but with early returns each case is independent)
                    self.current_scope_mut().defined = before_snapshot.clone();
                }
            }
            SwitchBody::ColonDelimited(block) => {
                for case in block.cases.iter() {
                    // Analyze this case as a separate branch
                    for stmt in case.statements().iter() {
                        self.analyze_statement(stmt);
                    }

                    // Save the state after this case
                    branch_snapshots.push(self.current_scope().snapshot());

                    // Reset to before state for next case
                    self.current_scope_mut().defined = before_snapshot.clone();
                }
            }
        }

        // Reset to original state before merging
        self.current_scope_mut().defined = before_snapshot.clone();

        // Merge branch results
        // Note: If there's no default case, we consider the "no match" path
        // For now, we conservatively assume a default case might not exist
        self.merge_branches(&before_snapshot, branch_snapshots);
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
                } else if !name.starts_with("$$") {
                    // Check if it's defined or possibly defined
                    if self.is_possibly_defined(name) {
                        // Variable is defined in some branches but not all
                        let (line, col) = self.get_line_col(var.span().start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "undefined.variable",
                                format!("Variable {} might not be defined.", name),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("variable.possiblyUndefined"),
                        );
                    } else if !self.is_defined(name) {
                        // Variable is completely undefined
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
