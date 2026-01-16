//! Check for invalid binary operations (Level 4)
//!
//! Detects binary operations with incompatible operand types.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::*;
use std::path::PathBuf;

/// Checks for invalid binary operations
pub struct InvalidBinaryOpCheck;

impl Check for InvalidBinaryOpCheck {
    fn id(&self) -> &'static str {
        "binaryOp.invalid"
    }

    fn description(&self) -> &'static str {
        "Detects binary operations with incompatible operand types"
    }

    fn level(&self) -> u8 {
        4
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = BinaryOpAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };

        analyzer.analyze_program(program);
        analyzer.issues
    }
}

struct BinaryOpAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExprType {
    Int,
    Float,
    String,
    Array,
    Object,
    Bool,
    Null,
    Unknown,
}

impl<'s> BinaryOpAnalyzer<'s> {
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

    /// Infer the type of an expression (simplified)
    fn infer_type(&self, expr: &Expression) -> ExprType {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Integer(_) => ExprType::Int,
                Literal::Float(_) => ExprType::Float,
                Literal::String(_) => ExprType::String,
                Literal::True(_) | Literal::False(_) => ExprType::Bool,
                Literal::Null(_) => ExprType::Null,
            },
            Expression::Array(_) | Expression::LegacyArray(_) => ExprType::Array,
            Expression::Instantiation(_) => ExprType::Object,
            Expression::Binary(binary) => {
                // Infer result type of binary operations
                match &binary.operator {
                    BinaryOperator::Addition(_)
                    | BinaryOperator::Subtraction(_)
                    | BinaryOperator::Multiplication(_)
                    | BinaryOperator::Division(_)
                    | BinaryOperator::Modulo(_)
                    | BinaryOperator::Exponentiation(_) => {
                        let lhs = self.infer_type(&binary.lhs);
                        let rhs = self.infer_type(&binary.rhs);
                        if lhs == ExprType::Float || rhs == ExprType::Float {
                            ExprType::Float
                        } else {
                            ExprType::Int
                        }
                    }
                    BinaryOperator::StringConcat(_) => ExprType::String,
                    BinaryOperator::Equal(_)
                    | BinaryOperator::NotEqual(_)
                    | BinaryOperator::Identical(_)
                    | BinaryOperator::NotIdentical(_)
                    | BinaryOperator::LessThan(_)
                    | BinaryOperator::LessThanOrEqual(_)
                    | BinaryOperator::GreaterThan(_)
                    | BinaryOperator::GreaterThanOrEqual(_)
                    | BinaryOperator::And(_)
                    | BinaryOperator::Or(_) => ExprType::Bool,
                    _ => ExprType::Unknown,
                }
            }
            _ => ExprType::Unknown,
        }
    }

    /// Check if a type is numeric (int or float)
    fn is_numeric(&self, ty: ExprType) -> bool {
        matches!(ty, ExprType::Int | ExprType::Float)
    }

    /// Check if a binary operation is valid
    fn check_binary_op(&mut self, binary: &Binary) {
        let lhs_type = self.infer_type(&binary.lhs);
        let rhs_type = self.infer_type(&binary.rhs);

        // Skip if we can't infer types
        if lhs_type == ExprType::Unknown || rhs_type == ExprType::Unknown {
            return;
        }

        let op_span = match &binary.operator {
            BinaryOperator::Addition(s)
            | BinaryOperator::Subtraction(s)
            | BinaryOperator::Multiplication(s)
            | BinaryOperator::Division(s)
            | BinaryOperator::Modulo(s)
            | BinaryOperator::Exponentiation(s)
            | BinaryOperator::BitwiseAnd(s)
            | BinaryOperator::BitwiseOr(s)
            | BinaryOperator::BitwiseXor(s)
            | BinaryOperator::LeftShift(s)
            | BinaryOperator::RightShift(s) => s,
            _ => return, // Other operators are more permissive
        };

        let error_msg = match &binary.operator {
            BinaryOperator::Addition(_) => {
                // Array + Array is valid (array union)
                if lhs_type == ExprType::Array && rhs_type == ExprType::Array {
                    return;
                }
                // Otherwise, both operands should be numeric
                if !self.is_numeric(lhs_type) {
                    Some(format!(
                        "Binary operation \"+\" between {:?} and {:?} results in an error.",
                        lhs_type, rhs_type
                    ))
                } else if !self.is_numeric(rhs_type) {
                    Some(format!(
                        "Binary operation \"+\" between {:?} and {:?} results in an error.",
                        lhs_type, rhs_type
                    ))
                } else {
                    None
                }
            }
            BinaryOperator::Subtraction(_)
            | BinaryOperator::Multiplication(_)
            | BinaryOperator::Division(_)
            | BinaryOperator::Modulo(_)
            | BinaryOperator::Exponentiation(_) => {
                let op_str = match &binary.operator {
                    BinaryOperator::Subtraction(_) => "-",
                    BinaryOperator::Multiplication(_) => "*",
                    BinaryOperator::Division(_) => "/",
                    BinaryOperator::Modulo(_) => "%",
                    BinaryOperator::Exponentiation(_) => "**",
                    _ => unreachable!(),
                };

                if !self.is_numeric(lhs_type) || !self.is_numeric(rhs_type) {
                    Some(format!(
                        "Binary operation \"{}\" between {:?} and {:?} results in an error.",
                        op_str, lhs_type, rhs_type
                    ))
                } else {
                    None
                }
            }
            BinaryOperator::BitwiseAnd(_)
            | BinaryOperator::BitwiseOr(_)
            | BinaryOperator::BitwiseXor(_)
            | BinaryOperator::LeftShift(_)
            | BinaryOperator::RightShift(_) => {
                let op_str = match &binary.operator {
                    BinaryOperator::BitwiseAnd(_) => "&",
                    BinaryOperator::BitwiseOr(_) => "|",
                    BinaryOperator::BitwiseXor(_) => "^",
                    BinaryOperator::LeftShift(_) => "<<",
                    BinaryOperator::RightShift(_) => ">>",
                    _ => unreachable!(),
                };

                // Bitwise operations require integers
                if lhs_type != ExprType::Int || rhs_type != ExprType::Int {
                    Some(format!(
                        "Binary operation \"{}\" between {:?} and {:?} results in an error.",
                        op_str, lhs_type, rhs_type
                    ))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(msg) = error_msg {
            let (line, col) = self.get_line_col(op_span.start.offset as usize);
            self.issues.push(
                Issue::error("binaryOp.invalid", msg, self.file_path.clone(), line, col)
                    .with_identifier("binaryOp.invalid"),
            );
        }
    }

    fn analyze_program(&mut self, program: &Program) {
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
                self.check_binary_op(binary);
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
            Expression::Instantiation(inst) => {
                if let Some(arg_list) = &inst.argument_list {
                    for arg in arg_list.arguments.iter() {
                        self.analyze_expression(arg.value());
                    }
                }
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
    fn test_invalid_binary_op_check_level() {
        let check = InvalidBinaryOpCheck;
        assert_eq!(check.level(), 4);
    }
}
