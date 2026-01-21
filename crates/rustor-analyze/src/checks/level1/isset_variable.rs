//! Check for redundant isset() on variables that are always defined (Level 1)
//!
//! Detects isset() calls on variables that are guaranteed to exist.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashSet;
use std::path::PathBuf;

/// Checks for isset() on variables that are always defined
pub struct IssetVariableCheck;

impl Check for IssetVariableCheck {
    fn id(&self) -> &'static str {
        "isset.variable"
    }

    fn description(&self) -> &'static str {
        "Detects isset() calls on variables that always exist"
    }

    fn level(&self) -> u8 {
        1
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = IssetAnalyzer {
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
    /// Variables that are definitely non-nullable (assigned from non-null literals)
    definitely_non_nullable: HashSet<String>,
    /// Whether $this is available in this scope
    has_this: bool,
}

impl Scope {
    fn new() -> Self {
        Self {
            definitely_non_nullable: HashSet::new(),
            has_this: false,
        }
    }

    fn define_non_nullable(&mut self, name: String) {
        self.definitely_non_nullable.insert(name);
    }

    fn undefine(&mut self, name: &str) {
        self.definitely_non_nullable.remove(name);
    }

    fn is_definitely_non_nullable(&self, name: &str) -> bool {
        self.definitely_non_nullable.contains(name)
    }
}

struct IssetAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    scopes: Vec<Scope>,
    issues: Vec<Issue>,
}

impl<'s> IssetAnalyzer<'s> {
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

    fn current_scope(&self) -> &Scope {
        self.scopes.last().unwrap()
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_superglobals(&mut self) {
        // Superglobals themselves are non-nullable (the arrays always exist)
        // BUT their contents/keys can be missing, so isset($_GET['key']) is valid
        // Don't mark them as non-nullable since isset() on superglobals is checking keys
        // not the superglobal itself
    }

    fn define_non_nullable(&mut self, name: String) {
        self.current_scope_mut().define_non_nullable(name);
    }

    fn undefine(&mut self, name: &str) {
        // Remove from all scopes (variable could be reassigned to nullable)
        for scope in self.scopes.iter_mut() {
            scope.undefine(name);
        }
    }

    fn is_definitely_non_nullable(&self, name: &str) -> bool {
        // Check all scopes from innermost to outermost
        for scope in self.scopes.iter().rev() {
            if scope.is_definitely_non_nullable(name) {
                return true;
            }
        }
        false
    }

    /// Check if an expression is a non-nullable literal
    fn is_non_nullable_literal<'a>(&self, expr: &Expression<'a>) -> bool {
        match expr {
            // String, int, float literals are non-nullable
            Expression::Literal(lit) => {
                matches!(lit,
                    Literal::Integer(_) |
                    Literal::Float(_) |
                    Literal::String(_) |
                    Literal::True(_) |
                    Literal::False(_)
                )
            }
            // Array literals are non-nullable
            Expression::Array(_) | Expression::LegacyArray(_) => true,
            // 'new' expressions return non-null objects
            Expression::Instantiation(_) => true,
            // Parenthesized expression - check inner
            Expression::Parenthesized(p) => self.is_non_nullable_literal(&p.expression),
            // Everything else (function calls, method calls, etc.) could be nullable
            _ => false,
        }
    }

    fn get_var_name<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Variable(Variable::Direct(var)) => {
                Some(self.get_span_text(&var.span).to_string())
            }
            _ => None,
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
                // Track assignments - only mark as non-nullable if RHS is a non-null literal
                if let Expression::Assignment(assign) = &expr_stmt.expression {
                    if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                        let var_name = self.get_span_text(&var.span).to_string();
                        if self.is_non_nullable_literal(&assign.rhs) {
                            self.define_non_nullable(var_name);
                        } else {
                            // Variable could be null - remove from non-nullable set
                            self.undefine(&var_name);
                        }
                    }
                }
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
                    // Track variable definitions in for loop initialization
                    if let Expression::Assignment(assign) = init {
                        if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                            let var_name = self.get_span_text(&var.span).to_string();
                            if self.is_non_nullable_literal(&assign.rhs) {
                                self.define_non_nullable(var_name);
                            } else {
                                self.undefine(&var_name);
                            }
                        }
                    }
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
                // Loop variables could be null depending on array contents - don't mark as non-nullable
                self.analyze_foreach_body(&foreach.body);
            }
            Statement::Function(func) => {
                self.push_scope();
                // Parameters can be nullable - don't mark as non-nullable
                // (would need type information to know if nullable)
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
                }
                self.pop_scope();
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        self.push_scope();
                        // $this is available in methods and is non-nullable
                        self.current_scope_mut().has_this = true;
                        self.define_non_nullable("$this".to_string());

                        // Parameters can be nullable - don't mark as non-nullable
                        // (would need type information to know if nullable)

                        if let MethodBody::Concrete(body) = &method.body {
                            for stmt in body.statements.iter() {
                                self.analyze_statement(stmt);
                            }
                        }
                        self.pop_scope();
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
            // Check isset() construct
            Expression::Construct(Construct::Isset(isset)) => {
                for value in isset.values.iter() {
                    // Only flag simple variable access, not array access like isset($arr['key'])
                    if let Expression::Variable(Variable::Direct(var)) = value {
                        let var_name = self.get_span_text(&var.span).to_string();
                        // Check if variable was assigned a non-nullable value
                        if self.is_definitely_non_nullable(&var_name) {
                            let (line, col) = self.get_line_col(value.span().start.offset as usize);
                            self.issues.push(
                                Issue::error(
                                    "isset.variable",
                                    format!(
                                        "Variable {} in isset() always exists and is not nullable.",
                                        var_name
                                    ),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("isset.variable"),
                            );
                        }
                    }
                    // Array access like isset($arr['key']) is always valid - skip
                }
            }
            Expression::Binary(binary) => {
                self.analyze_expression(&binary.lhs);
                self.analyze_expression(&binary.rhs);
            }
            Expression::UnaryPrefix(unary) => {
                self.analyze_expression(&unary.operand);
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
    fn test_isset_variable_check_level() {
        let check = IssetVariableCheck;
        assert_eq!(check.level(), 1);
    }
}
