//! Check for redundant type checks after type narrowing (Level 6)
//!
//! Detects type checks that are redundant because the type was already narrowed
//! in an outer scope.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::path::PathBuf;

/// Checks for redundant type checks after narrowing
pub struct AlreadyNarrowedTypeCheck;

impl Check for AlreadyNarrowedTypeCheck {
    fn id(&self) -> &'static str {
        "function.alreadyNarrowedType"
    }

    fn description(&self) -> &'static str {
        "Detects redundant type checks after type narrowing"
    }

    fn level(&self) -> u8 {
        6
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = NarrowedTypeAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            type_checks: Vec::new(), // Stack of active type checks
            issues: Vec::new(),
        };

        analyzer.analyze_program(program);
        analyzer.issues
    }
}

/// Represents a type check condition
#[derive(Debug, Clone, PartialEq)]
enum TypeCheck {
    Instanceof { var: String, class: String },
    IsString { var: String },
    IsInt { var: String },
    IsFloat { var: String },
    IsBool { var: String },
    IsArray { var: String },
    IsObject { var: String },
    IsNull { var: String },
    MethodExists { var: String, method: String },
}

impl TypeCheck {
    fn var_name(&self) -> &str {
        match self {
            TypeCheck::Instanceof { var, .. }
            | TypeCheck::IsString { var }
            | TypeCheck::IsInt { var }
            | TypeCheck::IsFloat { var }
            | TypeCheck::IsBool { var }
            | TypeCheck::IsArray { var }
            | TypeCheck::IsObject { var }
            | TypeCheck::IsNull { var }
            | TypeCheck::MethodExists { var, .. } => var,
        }
    }

    fn description(&self) -> String {
        match self {
            TypeCheck::Instanceof { var, class } => {
                format!("{} instanceof {}", var, class)
            }
            TypeCheck::IsString { var } => format!("is_string({})", var),
            TypeCheck::IsInt { var } => format!("is_int({})", var),
            TypeCheck::IsFloat { var } => format!("is_float({})", var),
            TypeCheck::IsBool { var } => format!("is_bool({})", var),
            TypeCheck::IsArray { var } => format!("is_array({})", var),
            TypeCheck::IsObject { var } => format!("is_object({})", var),
            TypeCheck::IsNull { var } => format!("is_null({})", var),
            TypeCheck::MethodExists { var, method } => {
                format!("method_exists({}, '{}')", var, method)
            }
        }
    }
}

struct NarrowedTypeAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    type_checks: Vec<TypeCheck>, // Stack of active type narrowing conditions
    issues: Vec<Issue>,
}

impl<'s> NarrowedTypeAnalyzer<'s> {
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

    /// Extract variable name from an expression
    fn extract_var_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Variable(Variable::Direct(var)) => {
                Some(self.get_span_text(&var.span).to_string())
            }
            _ => None,
        }
    }

    /// Try to parse a type check from an expression
    fn parse_type_check(&self, expr: &Expression) -> Option<TypeCheck> {
        match expr {
            // instanceof check
            Expression::Binary(binary) if matches!(binary.operator, BinaryOperator::Instanceof(_)) => {
                let var_name = self.extract_var_name(&binary.lhs)?;
                if let Expression::Identifier(ident) = &*binary.rhs {
                    let class_name = self.get_span_text(&ident.span()).to_string();
                    return Some(TypeCheck::Instanceof {
                        var: var_name,
                        class: class_name,
                    });
                }
                None
            }

            // is_* function calls
            Expression::Call(Call::Function(func_call)) => {
                if let Expression::Identifier(ident) = &*func_call.function {
                    let func_name = self.get_span_text(&ident.span()).to_lowercase();

                    // Get first argument
                    if func_call.argument_list.arguments.is_empty() {
                        return None;
                    }
                    let first_arg = func_call.argument_list.arguments.iter().next()?;
                    let var_name = self.extract_var_name(first_arg.value())?;

                    match func_name.as_str() {
                        "is_string" => Some(TypeCheck::IsString { var: var_name }),
                        "is_int" | "is_integer" | "is_long" => {
                            Some(TypeCheck::IsInt { var: var_name })
                        }
                        "is_float" | "is_double" | "is_real" => {
                            Some(TypeCheck::IsFloat { var: var_name })
                        }
                        "is_bool" => Some(TypeCheck::IsBool { var: var_name }),
                        "is_array" => Some(TypeCheck::IsArray { var: var_name }),
                        "is_object" => Some(TypeCheck::IsObject { var: var_name }),
                        "is_null" => Some(TypeCheck::IsNull { var: var_name }),
                        "method_exists" => {
                            // method_exists($obj, 'methodName')
                            if func_call.argument_list.arguments.len() >= 2 {
                                let method_arg = func_call.argument_list.arguments.iter().nth(1)?;
                                if let Expression::Literal(Literal::String(s)) =
                                    method_arg.value()
                                {
                                    let method_name = self.get_span_text(&s.span()).to_string();
                                    // Remove quotes
                                    let method_name = method_name.trim_matches(|c| c == '"' || c == '\'');
                                    return Some(TypeCheck::MethodExists {
                                        var: var_name,
                                        method: method_name.to_string(),
                                    });
                                }
                            }
                            None
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    /// Check if this type check is redundant given the current narrowing stack
    fn check_redundant_narrowing(&mut self, type_check: &TypeCheck, span: &mago_span::Span) {
        // Check if the same type check already exists in the stack
        for existing in &self.type_checks {
            if existing == type_check {
                let (line, col) = self.get_line_col(span.start.offset as usize);
                let msg = format!(
                    "Call to {} is already checked on line above, this condition is always true.",
                    type_check.description()
                );
                self.issues.push(
                    Issue::error(
                        "function.alreadyNarrowedType",
                        msg,
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("function.alreadyNarrowedType"),
                );
                return;
            }
        }
    }

    fn analyze_program(&mut self, program: &Program) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::If(if_stmt) => {
                // Check if the condition is a type check
                if let Some(type_check) = self.parse_type_check(&if_stmt.condition) {
                    // Check if it's redundant
                    self.check_redundant_narrowing(&type_check, &if_stmt.condition.span());

                    // Push the type check onto the stack
                    self.type_checks.push(type_check.clone());

                    // Analyze the body with the type check active
                    self.analyze_if_body(&if_stmt.body);

                    // Pop the type check
                    self.type_checks.pop();
                } else {
                    // Not a simple type check, just analyze the condition and body
                    self.analyze_expression(&if_stmt.condition);
                    self.analyze_if_body(&if_stmt.body);
                }
            }
            Statement::Expression(expr_stmt) => {
                self.analyze_expression(&expr_stmt.expression);
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
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::Function(func) => {
                // Reset type checks for new function scope
                let saved_checks = self.type_checks.clone();
                self.type_checks.clear();

                for stmt in func.body.statements.iter() {
                    self.analyze_statement(stmt);
                }

                self.type_checks = saved_checks;
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            // Reset type checks for new method scope
                            let saved_checks = self.type_checks.clone();
                            self.type_checks.clear();

                            for stmt in body.statements.iter() {
                                self.analyze_statement(stmt);
                            }

                            self.type_checks = saved_checks;
                        }
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
            _ => {}
        }
    }

    fn analyze_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
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
            Expression::Call(call) => match call {
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
            },
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
    fn test_already_narrowed_type_check_level() {
        let check = AlreadyNarrowedTypeCheck;
        assert_eq!(check.level(), 6);
    }
}
