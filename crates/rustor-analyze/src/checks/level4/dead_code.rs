//! Dead code detection (Level 4)
//!
//! Detects:
//! - Unreachable statements after return/throw
//! - Always-false instanceof checks
//! - Always-true type narrowing (is_string on string, etc.)
//! - Always-true/false comparisons

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Check for dead/unreachable code
pub struct DeadCodeCheck;

impl Check for DeadCodeCheck {
    fn id(&self) -> &'static str {
        "deadCode.unreachable"
    }

    fn description(&self) -> &'static str {
        "Detects unreachable code and always-true/false type checks"
    }

    fn level(&self) -> u8 {
        4
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = DeadCodeVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            param_types: HashMap::new(),
            variable_types: HashMap::new(),
            issues: Vec::new(),
        };

        visitor.analyze_program(program);
        visitor.issues
    }
}

struct DeadCodeVisitor<'s> {
    source: &'s str,
    file_path: PathBuf,
    /// Parameter types in current function scope
    param_types: HashMap<String, String>,
    /// Variable types from assignments
    variable_types: HashMap<String, String>,
    issues: Vec<Issue>,
}

impl<'s> DeadCodeVisitor<'s> {
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
                // Extract parameter types
                self.param_types.clear();
                for param in func.parameter_list.parameters.iter() {
                    if let Some(hint) = &param.hint {
                        if let Some(type_name) = self.extract_type_name(hint) {
                            let var_name = self.get_span_text(&param.variable.span).to_string();
                            self.param_types.insert(var_name, type_name);
                        }
                    }
                }

                // Check for unreachable code in function body
                self.check_block_for_unreachable(&func.body);

                // Analyze function body
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
                }

                self.param_types.clear();
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            // Extract parameter types
                            self.param_types.clear();
                            for param in method.parameter_list.parameters.iter() {
                                if let Some(hint) = &param.hint {
                                    if let Some(type_name) = self.extract_type_name(hint) {
                                        let var_name =
                                            self.get_span_text(&param.variable.span).to_string();
                                        self.param_types.insert(var_name, type_name);
                                    }
                                }
                            }

                            // Check for unreachable code
                            self.check_block_for_unreachable(body);

                            // Analyze method body
                            for inner in body.statements.iter() {
                                self.analyze_statement(inner);
                            }

                            self.param_types.clear();
                        }
                    }
                }
            }
            Statement::If(if_stmt) => {
                self.check_expression(&if_stmt.condition);
                self.analyze_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.check_expression(&while_stmt.condition);
                self.analyze_while_body(&while_stmt.body);
            }
            Statement::For(for_stmt) => {
                self.analyze_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.analyze_foreach_body(&foreach.body);
            }
            Statement::Switch(switch) => {
                // Check for unreachable code after return in cases
                match &switch.body {
                    SwitchBody::BraceDelimited(body) => {
                        for case in body.cases.iter() {
                            match case {
                                SwitchCase::Expression(expr_case) => {
                                    self.check_statements_for_unreachable(&expr_case.statements);
                                }
                                SwitchCase::Default(default_case) => {
                                    self.check_statements_for_unreachable(&default_case.statements);
                                }
                            }
                        }
                    }
                    SwitchBody::ColonDelimited(body) => {
                        for case in body.cases.iter() {
                            match case {
                                SwitchCase::Expression(expr_case) => {
                                    self.check_statements_for_unreachable(&expr_case.statements);
                                }
                                SwitchCase::Default(default_case) => {
                                    self.check_statements_for_unreachable(&default_case.statements);
                                }
                            }
                        }
                    }
                }
            }
            Statement::Try(try_stmt) => {
                self.check_block_for_unreachable(&try_stmt.block);
                for catch in try_stmt.catch_clauses.iter() {
                    self.check_block_for_unreachable(&catch.block);
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    self.check_block_for_unreachable(&finally.block);
                }
            }
            Statement::Block(block) => {
                self.check_block_for_unreachable(block);
            }
            Statement::Expression(expr_stmt) => {
                self.check_expression(&expr_stmt.expression);
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

    /// Check a block for unreachable code after return/throw
    fn check_block_for_unreachable<'a>(&mut self, block: &Block<'a>) {
        self.check_statements_for_unreachable(&block.statements);
    }

    /// Check a sequence of statements for unreachable code
    fn check_statements_for_unreachable<'a>(&mut self, statements: &Sequence<'a, Statement<'a>>) {
        let mut found_terminator = false;
        let mut _terminator_span: Option<mago_span::Span> = None;

        for stmt in statements.iter() {
            if found_terminator {
                // This statement is unreachable
                let span = stmt.span();
                let (line, col) = self.get_line_col(span.start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "deadCode.unreachable",
                        "Unreachable statement - code above always terminates.".to_string(),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("deadCode.unreachable"),
                );
                // Only report first unreachable statement
                break;
            }

            // Check if this statement is a terminator
            if self.is_terminator(stmt) {
                found_terminator = true;
                _terminator_span = Some(stmt.span());
            }
        }
    }

    /// Check if a statement always terminates (return, throw, exit, etc.)
    fn is_terminator<'a>(&self, stmt: &Statement<'a>) -> bool {
        match stmt {
            Statement::Return(_) => true,
            Statement::Continue(_) => true,
            Statement::Break(_) => true,
            Statement::Expression(expr_stmt) => {
                // Check for throw expression (PHP 8+)
                if matches!(expr_stmt.expression, Expression::Throw(_)) {
                    return true;
                }
                // Check for exit/die calls
                if let Expression::Call(Call::Function(func_call)) = &expr_stmt.expression {
                    if let Expression::Identifier(ident) = &*func_call.function {
                        let name = self.get_span_text(&ident.span()).to_lowercase();
                        return name == "exit" || name == "die";
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn analyze_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                if let Statement::Block(block) = &*stmt_body.statement {
                    self.check_block_for_unreachable(block);
                }
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                }
            }
            IfBody::ColonDelimited(block) => {
                self.check_statements_for_unreachable(&block.statements);
            }
        }
    }

    fn analyze_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => {
                if let Statement::Block(block) = &**stmt {
                    self.check_block_for_unreachable(block);
                }
            }
            WhileBody::ColonDelimited(block) => {
                self.check_statements_for_unreachable(&block.statements);
            }
        }
    }

    fn analyze_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => {
                if let Statement::Block(block) = &**stmt {
                    self.check_block_for_unreachable(block);
                }
            }
            ForBody::ColonDelimited(block) => {
                self.check_statements_for_unreachable(&block.statements);
            }
        }
    }

    fn analyze_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => {
                if let Statement::Block(block) = &**stmt {
                    self.check_block_for_unreachable(block);
                }
            }
            ForeachBody::ColonDelimited(block) => {
                self.check_statements_for_unreachable(&block.statements);
            }
        }
    }

    /// Check expressions for dead code patterns
    fn check_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // Check for redundant type narrowing calls
            Expression::Call(Call::Function(func_call)) => {
                self.check_type_narrowing_call(func_call);
            }
            // Check comparisons and instanceof
            Expression::Binary(binary) => {
                // Check for always-false instanceof
                if binary.operator.is_instanceof() {
                    self.check_instanceof(binary);
                }
                self.check_binary_comparison(binary);
                // Recurse
                self.check_expression(&binary.lhs);
                self.check_expression(&binary.rhs);
            }
            // Recurse into other expressions
            Expression::Parenthesized(p) => self.check_expression(&p.expression),
            Expression::UnaryPrefix(p) => self.check_expression(&p.operand),
            Expression::Conditional(c) => {
                self.check_expression(&c.condition);
                if let Some(then) = &c.then {
                    self.check_expression(then);
                }
                self.check_expression(&c.r#else);
            }
            _ => {}
        }
    }

    /// Check for always-false instanceof (binary expression with instanceof operator)
    fn check_instanceof<'a>(&mut self, binary: &Binary<'a>) {
        // Get type of left side
        if let Some(var_type) = self.get_expression_type(&binary.lhs) {
            // Get class being checked
            if let Expression::Identifier(ident) = &*binary.rhs {
                let class_name = self.get_span_text(&ident.span());

                // Check if types are incompatible
                if self.is_instanceof_always_false(&var_type, class_name) {
                    let (line, col) =
                        self.get_line_col(binary.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "instanceof.alwaysFalse",
                            format!(
                                "Instanceof between {} and {} will always evaluate to false.",
                                var_type, class_name
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("instanceof.alwaysFalse"),
                    );
                }
            }
        }
    }

    /// Check if instanceof will always be false
    fn is_instanceof_always_false(&self, var_type: &str, class_name: &str) -> bool {
        let var_lower = var_type.to_lowercase();
        let _class_lower = class_name.to_lowercase();

        // Scalar types can never be instances of classes
        let scalar_types = ["string", "int", "float", "bool", "array", "null"];
        if scalar_types.contains(&var_lower.as_str()) {
            return true;
        }

        false
    }

    /// Check for redundant type narrowing calls like is_string($string)
    fn check_type_narrowing_call<'a>(&mut self, func_call: &FunctionCall<'a>) {
        if let Expression::Identifier(ident) = &*func_call.function {
            let func_name = self.get_span_text(&ident.span()).to_lowercase();

            // Map of type-checking functions to their expected types
            let type_checks: &[(&str, &str)] = &[
                ("is_string", "string"),
                ("is_int", "int"),
                ("is_integer", "int"),
                ("is_float", "float"),
                ("is_double", "float"),
                ("is_bool", "bool"),
                ("is_array", "array"),
                ("is_null", "null"),
                ("is_object", "object"),
                ("is_callable", "callable"),
                ("is_numeric", "int"),
                ("is_resource", "resource"),
            ];

            for (check_func, expected_type) in type_checks {
                if func_name == *check_func {
                    // Get first argument
                    if let Some(first_arg) = func_call.argument_list.arguments.first() {
                        if let Some(arg_type) = self.get_expression_type(first_arg.value()) {
                            let arg_lower = arg_type.to_lowercase();
                            if arg_lower == *expected_type {
                                let (line, col) =
                                    self.get_line_col(func_call.span().start.offset as usize);
                                self.issues.push(
                                    Issue::error(
                                        "function.alreadyNarrowedType",
                                        format!(
                                            "Call to function {}() with {} will always evaluate to true.",
                                            func_name, arg_type
                                        ),
                                        self.file_path.clone(),
                                        line,
                                        col,
                                    )
                                    .with_identifier("function.alreadyNarrowedType"),
                                );
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    /// Check binary comparisons for always-true/false patterns
    fn check_binary_comparison<'a>(&mut self, _binary: &Binary<'a>) {
        // Check for comparisons that are always true due to type constraints
        // e.g., $x <= 0 when $x is already constrained to be <= 0
        // This is complex and requires tracking value ranges, so we'll do basic checks

        // For now, check for simple patterns in if-elseif chains
        // This would require more context to implement fully
    }

    /// Get the type of an expression from parameter types or variable assignments
    fn get_expression_type<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Variable(Variable::Direct(var)) => {
                let var_name = self.get_span_text(&var.span);
                // Check parameter types first
                if let Some(type_name) = self.param_types.get(var_name) {
                    return Some(type_name.clone());
                }
                // Check variable types
                self.variable_types.get(var_name).cloned()
            }
            Expression::Literal(literal) => {
                // Infer type from literal
                match literal {
                    Literal::String(_) => Some("string".to_string()),
                    Literal::Integer(_) => Some("int".to_string()),
                    Literal::Float(_) => Some("float".to_string()),
                    Literal::True(_) | Literal::False(_) => Some("bool".to_string()),
                    Literal::Null(_) => Some("null".to_string()),
                }
            }
            _ => None,
        }
    }

    /// Extract type name from a hint
    fn extract_type_name(&self, hint: &Hint<'_>) -> Option<String> {
        match hint {
            // Class/interface names
            Hint::Identifier(ident) => Some(self.get_span_text(&ident.span()).to_string()),
            Hint::Nullable(nullable) => self.extract_type_name(&nullable.hint),
            Hint::Parenthesized(p) => self.extract_type_name(&p.hint),
            // Built-in types
            Hint::Integer(_) => Some("int".to_string()),
            Hint::String(_) => Some("string".to_string()),
            Hint::Float(_) => Some("float".to_string()),
            Hint::Bool(_) => Some("bool".to_string()),
            Hint::Array(_) => Some("array".to_string()),
            Hint::Object(_) => Some("object".to_string()),
            Hint::Mixed(_) => Some("mixed".to_string()),
            Hint::Callable(_) => Some("callable".to_string()),
            Hint::Iterable(_) => Some("iterable".to_string()),
            Hint::Void(_) => Some("void".to_string()),
            Hint::Never(_) => Some("never".to_string()),
            Hint::Null(_) => Some("null".to_string()),
            Hint::True(_) => Some("true".to_string()),
            Hint::False(_) => Some("false".to_string()),
            Hint::Union(union) => self.extract_type_name(union.left),
            Hint::Intersection(intersection) => self.extract_type_name(intersection.left),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dead_code_check_level() {
        let check = DeadCodeCheck;
        assert_eq!(check.level(), 4);
    }
}
