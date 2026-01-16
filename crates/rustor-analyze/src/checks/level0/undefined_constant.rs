//! Check for undefined global constants (Level 0)
//!
//! Detects usage of undefined global constants like MY_CONSTANT.
//! Does NOT check class constants (see class_constant.rs).

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashSet;
use std::path::PathBuf;

/// Checks for undefined global constant usage
pub struct UndefinedConstantCheck;

impl Check for UndefinedConstantCheck {
    fn id(&self) -> &'static str {
        "constant.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects usage of undefined global constants"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = UndefinedConstantVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            defined_constants: HashSet::new(),
            issues: Vec::new(),
        };

        // First pass: collect constant definitions
        visitor.collect_definitions(program);

        // Second pass: check constant usage
        visitor.check_program(program);

        visitor.issues
    }
}

struct UndefinedConstantVisitor<'s> {
    source: &'s str,
    file_path: PathBuf,
    defined_constants: HashSet<String>,
    issues: Vec<Issue>,
}

impl<'s> UndefinedConstantVisitor<'s> {
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

    /// Collect all constant definitions (define() calls and const declarations)
    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        // Add PHP built-in constants
        self.add_builtin_constants();

        // Collect user-defined constants
        for stmt in program.statements.iter() {
            self.collect_from_stmt(stmt);
        }
    }

    fn add_builtin_constants(&mut self) {
        // PHP built-in constants
        let builtins = [
            "TRUE", "FALSE", "NULL",
            "PHP_VERSION", "PHP_OS", "PHP_SAPI",
            "PHP_EOL", "PHP_INT_MAX", "PHP_INT_MIN",
            "E_ERROR", "E_WARNING", "E_PARSE", "E_NOTICE",
            "E_CORE_ERROR", "E_CORE_WARNING", "E_COMPILE_ERROR",
            "E_COMPILE_WARNING", "E_USER_ERROR", "E_USER_WARNING",
            "E_USER_NOTICE", "E_STRICT", "E_RECOVERABLE_ERROR",
            "E_DEPRECATED", "E_USER_DEPRECATED", "E_ALL",
            "DIRECTORY_SEPARATOR", "PATH_SEPARATOR",
            "__FILE__", "__LINE__", "__DIR__", "__FUNCTION__",
            "__CLASS__", "__METHOD__", "__NAMESPACE__",
            // Common third-party constants
            "STDIN", "STDOUT", "STDERR",
        ];

        for constant in &builtins {
            self.defined_constants.insert(constant.to_string());
            // Also add lowercase version for case-insensitive matching
            self.defined_constants.insert(constant.to_lowercase());
        }
    }

    fn collect_from_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            // Recurse into namespaces and blocks
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_stmt(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_stmt(inner);
                    }
                }
            },
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_from_stmt(inner);
                }
            }
            // Check for define() calls in expression statements
            Statement::Expression(expr_stmt) => {
                self.collect_from_expression(&expr_stmt.expression);
            }
            _ => {}
        }
    }

    fn collect_from_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // define('CONSTANT_NAME', value);
            Expression::Call(Call::Function(call)) => {
                // Check if this is a define() call
                if let Expression::Identifier(ident) = &*call.function {
                    let func_name = self.get_span_text(&ident.span());
                    if func_name.eq_ignore_ascii_case("define") {
                        // First argument should be the constant name (string literal)
                        if let Some(first_arg) = call.argument_list.arguments.first() {
                            if let Expression::Literal(Literal::String(s)) = first_arg.value() {
                                let const_name = self.get_span_text(&s.span());
                                // Remove quotes
                                let const_name = const_name.trim_matches(|c| c == '"' || c == '\'');
                                self.defined_constants.insert(const_name.to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Check all constant usage in the program
    fn check_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.check_expression(&expr_stmt.expression);
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.check_expression(value);
                }
            }
            Statement::If(if_stmt) => {
                self.check_expression(&if_stmt.condition);
                self.check_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.check_expression(&while_stmt.condition);
                self.check_while_body(&while_stmt.body);
            }
            Statement::For(for_stmt) => {
                for init in for_stmt.initializations.iter() {
                    self.check_expression(init);
                }
                for cond in for_stmt.conditions.iter() {
                    self.check_expression(cond);
                }
                for inc in for_stmt.increments.iter() {
                    self.check_expression(inc);
                }
                self.check_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.check_expression(&foreach.expression);
                self.check_foreach_body(&foreach.body);
            }
            Statement::Switch(switch) => {
                self.check_expression(&switch.expression);
                match &switch.body {
                    SwitchBody::BraceDelimited(body) => {
                        for case in body.cases.iter() {
                            match case {
                                SwitchCase::Expression(c) => {
                                    self.check_expression(&c.expression);
                                    for stmt in c.statements.iter() {
                                        self.check_statement(stmt);
                                    }
                                }
                                SwitchCase::Default(d) => {
                                    for stmt in d.statements.iter() {
                                        self.check_statement(stmt);
                                    }
                                }
                            }
                        }
                    }
                    SwitchBody::ColonDelimited(body) => {
                        for case in body.cases.iter() {
                            match case {
                                SwitchCase::Expression(c) => {
                                    self.check_expression(&c.expression);
                                    for stmt in c.statements.iter() {
                                        self.check_statement(stmt);
                                    }
                                }
                                SwitchCase::Default(d) => {
                                    for stmt in d.statements.iter() {
                                        self.check_statement(stmt);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Statement::Echo(echo) => {
                for value in echo.values.iter() {
                    self.check_expression(value);
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner);
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.check_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.check_statement(inner);
                    }
                }
            },
            Statement::Class(class) => {
                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Method(method) => {
                            if let MethodBody::Concrete(body) = &method.body {
                                for stmt in body.statements.iter() {
                                    self.check_statement(stmt);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::Function(func) => {
                for stmt in func.body.statements.iter() {
                    self.check_statement(stmt);
                }
            }
            _ => {}
        }
    }

    fn check_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // This is a constant fetch - check if it's defined
            Expression::ConstantAccess(const_access) => {
                // Get the constant name - use the span directly
                let name = self.get_span_text(&const_access.name.span());

                // Skip special keywords
                if matches!(name.to_lowercase().as_str(),
                           "true" | "false" | "null") {
                    return;
                }

                // Check if constant is defined (case-insensitive for PHP constants)
                if !self.defined_constants.contains(name) &&
                   !self.defined_constants.contains(&name.to_lowercase()) {
                    let (line, col) = self.get_line_col(const_access.name.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "constant.notFound",
                            format!("Constant {} not found.", name),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("constant.notFound"),
                    );
                } else {
                }
            }
            Expression::Identifier(ident) => {
                let name = self.get_span_text(&ident.span());
                // Identifiers in other contexts - skip for now
            }
            // Recurse into complex expressions
            Expression::Binary(binary) => {
                self.check_expression(&binary.lhs);
                self.check_expression(&binary.rhs);
            }
            Expression::UnaryPrefix(prefix) => {
                self.check_expression(&prefix.operand);
            }
            Expression::UnaryPostfix(postfix) => {
                self.check_expression(&postfix.operand);
            }
            Expression::Conditional(cond) => {
                self.check_expression(&cond.condition);
                if let Some(then) = &cond.then {
                    self.check_expression(then);
                }
                self.check_expression(&cond.r#else);
            }
            Expression::Assignment(assign) => {
                self.check_expression(&assign.lhs);
                self.check_expression(&assign.rhs);
            }
            Expression::Array(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.check_expression(&kv.key);
                            self.check_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.check_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.check_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::LegacyArray(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.check_expression(&kv.key);
                            self.check_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.check_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.check_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::Call(call) => {
                match call {
                    Call::Function(func_call) => {
                        // Don't check the function name itself as a constant
                        // Only check the arguments
                        for arg in func_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                    Call::Method(method_call) => {
                        self.check_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                    Call::NullSafeMethod(method_call) => {
                        self.check_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                    Call::StaticMethod(static_call) => {
                        for arg in static_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                }
            }
            Expression::Parenthesized(p) => {
                self.check_expression(&p.expression);
            }
            Expression::ArrayAccess(arr) => {
                self.check_expression(&arr.array);
                self.check_expression(&arr.index);
            }
            Expression::Construct(_) => {
            }
            other => {
            }
        }
    }

    fn check_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.check_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                    self.check_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.check_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                    for stmt in else_if.statements.iter() {
                        self.check_statement(stmt);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for stmt in else_clause.statements.iter() {
                        self.check_statement(stmt);
                    }
                }
            }
        }
    }

    fn check_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => self.check_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => self.check_statement(stmt),
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => self.check_statement(stmt),
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undefined_constant_check_level() {
        let check = UndefinedConstantCheck;
        assert_eq!(check.level(), 0);
    }
}
