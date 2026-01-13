//! Check for return type validation (Level 3)
//!
//! At level 3, PHPStan checks that return statements match the declared return type.
//! This includes:
//! - Functions/methods with declared return types returning wrong types
//! - Functions/methods with void return type returning values
//! - Functions/methods with non-void return type not returning values

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use mago_syntax::ast::access::Access;
use std::path::PathBuf;

/// Checks for return type validation
pub struct ReturnTypeCheck;

impl Check for ReturnTypeCheck {
    fn id(&self) -> &'static str {
        "return.type"
    }

    fn description(&self) -> &'static str {
        "Validates that return statements match declared return types"
    }

    fn level(&self) -> u8 {
        3
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = ReturnTypeAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
            current_class: None,
        };

        analyzer.analyze_program(program);
        analyzer.issues
    }
}

struct ReturnTypeAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
    /// Current class context for resolving 'self' and 'static'
    current_class: Option<String>,
}

impl<'s> ReturnTypeAnalyzer<'s> {
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
                let func_name = self.get_span_text(&func.name.span).to_string();
                let return_type = self.extract_return_type(&func.return_type_hint);
                self.analyze_function_body(&func.body, &func_name, return_type.as_deref(), false, func.span());
            }
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();
                // Save the current class context and set new one
                let prev_class = self.current_class.clone();
                self.current_class = Some(class_name.clone());

                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            let method_name = self.get_span_text(&method.name.span).to_string();
                            let return_type = self.extract_return_type(&method.return_type_hint);
                            let full_name = format!("{}::{}", class_name, method_name);
                            self.analyze_method_body(body, &full_name, return_type.as_deref(), method.span());
                        }
                    }
                }

                // Restore previous class context
                self.current_class = prev_class;
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

    fn extract_return_type(&self, hint: &Option<FunctionLikeReturnTypeHint<'_>>) -> Option<String> {
        hint.as_ref().map(|h| self.get_span_text(&h.hint.span()).to_string())
    }

    fn analyze_function_body<'a>(
        &mut self,
        body: &Block<'a>,
        func_name: &str,
        return_type: Option<&str>,
        _is_method: bool,
        _func_span: mago_span::Span,
    ) {
        // Check returns in block for type validation
        // Missing return check is handled by MissingReturnCheck at level 0
        self.check_returns_in_block(body, func_name, return_type);
    }

    fn analyze_method_body<'a>(
        &mut self,
        body: &Block<'a>,
        method_name: &str,
        return_type: Option<&str>,
        _method_span: mago_span::Span,
    ) {
        // Check returns in block for type validation
        // Missing return check is handled by MissingReturnCheck at level 0
        self.check_returns_in_block(body, method_name, return_type);
    }

    /// Check all return statements in a block and return true if any has a value
    fn check_returns_in_block<'a>(
        &mut self,
        block: &Block<'a>,
        func_name: &str,
        return_type: Option<&str>,
    ) -> bool {
        // Check if this is a generator function (contains yield)
        if self.block_contains_yield(block) {
            // Generator functions don't need explicit return statements
            return true;
        }

        let mut has_return_with_value = false;
        for stmt in block.statements.iter() {
            if self.check_return_stmt(stmt, func_name, return_type) {
                has_return_with_value = true;
            }
        }
        has_return_with_value
    }

    /// Check a single statement for return issues, returns true if return has value
    fn check_return_stmt<'a>(
        &mut self,
        stmt: &Statement<'a>,
        func_name: &str,
        return_type: Option<&str>,
    ) -> bool {
        match stmt {
            Statement::Return(ret) => {
                let has_value = ret.value.is_some();

                if let Some(rt) = return_type {
                    let rt_lower = rt.to_lowercase();

                    // void function returning a value
                    if rt_lower == "void" && has_value {
                        let span = ret.span();
                        let (line, col) = self.get_line_col(span.start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "return.type",
                                format!(
                                    "Function {} with return type void returns a value.",
                                    func_name
                                ),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("return.void"),
                        );
                    }

                    // non-void function returning without value
                    if rt_lower != "void" && rt_lower != "never" && !has_value {
                        let span = ret.span();
                        let (line, col) = self.get_line_col(span.start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "return.type",
                                format!(
                                    "Function {} with return type {} returns without a value.",
                                    func_name, rt
                                ),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("return.empty"),
                        );
                    }

                    // Type checking for return value
                    if has_value && rt_lower != "void" && rt_lower != "mixed" {
                        if let Some(value) = &ret.value {
                            self.check_return_value_type(value, rt, func_name, ret.span());
                        }
                    }
                }

                has_value
            }
            Statement::Block(block) => {
                let mut has_value = false;
                for inner in block.statements.iter() {
                    if self.check_return_stmt(inner, func_name, return_type) {
                        has_value = true;
                    }
                }
                has_value
            }
            Statement::If(if_stmt) => {
                self.check_returns_in_if_body(&if_stmt.body, func_name, return_type)
            }
            Statement::Try(try_stmt) => {
                let mut has_value = false;
                for inner in try_stmt.block.statements.iter() {
                    if self.check_return_stmt(inner, func_name, return_type) {
                        has_value = true;
                    }
                }
                for catch in try_stmt.catch_clauses.iter() {
                    for inner in catch.block.statements.iter() {
                        if self.check_return_stmt(inner, func_name, return_type) {
                            has_value = true;
                        }
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    for inner in finally.block.statements.iter() {
                        if self.check_return_stmt(inner, func_name, return_type) {
                            has_value = true;
                        }
                    }
                }
                has_value
            }
            _ => false,
        }
    }

    fn check_returns_in_if_body<'a>(
        &mut self,
        body: &IfBody<'a>,
        func_name: &str,
        return_type: Option<&str>,
    ) -> bool {
        let mut has_value = false;
        match body {
            IfBody::Statement(stmt_body) => {
                if self.check_return_stmt(stmt_body.statement, func_name, return_type) {
                    has_value = true;
                }
                for else_if in stmt_body.else_if_clauses.iter() {
                    if self.check_return_stmt(else_if.statement, func_name, return_type) {
                        has_value = true;
                    }
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    if self.check_return_stmt(else_clause.statement, func_name, return_type) {
                        has_value = true;
                    }
                }
            }
            IfBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    if self.check_return_stmt(inner, func_name, return_type) {
                        has_value = true;
                    }
                }
                for else_if in block.else_if_clauses.iter() {
                    for inner in else_if.statements.iter() {
                        if self.check_return_stmt(inner, func_name, return_type) {
                            has_value = true;
                        }
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        if self.check_return_stmt(inner, func_name, return_type) {
                            has_value = true;
                        }
                    }
                }
            }
        }
        has_value
    }

    /// Check if a return value type matches the declared return type
    fn check_return_value_type<'a>(
        &mut self,
        value: &Expression<'a>,
        expected_type: &str,
        func_name: &str,
        ret_span: mago_span::Span,
    ) {
        let actual_type = self.infer_expression_type(value);

        if let Some(actual) = actual_type {
            let expected_lower = expected_type.to_lowercase();
            let actual_lower = actual.to_lowercase();

            // Skip if types are compatible
            if self.types_compatible(&expected_lower, &actual_lower, self.current_class.as_deref()) {
                return;
            }

            let (line, col) = self.get_line_col(ret_span.start.offset as usize);
            self.issues.push(
                Issue::error(
                    "return.type",
                    format!(
                        "Function {} should return {} but returns {}.",
                        func_name, expected_type, actual
                    ),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("return.typeMismatch"),
            );
        }
    }

    /// Infer the type of an expression (basic inference)
    fn infer_expression_type<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::String(_) => Some("string".to_string()),
                Literal::Integer(_) => Some("int".to_string()),
                Literal::Float(_) => Some("float".to_string()),
                Literal::True(_) | Literal::False(_) => Some("bool".to_string()),
                Literal::Null(_) => Some("null".to_string()),
            },
            Expression::Array(_) | Expression::LegacyArray(_) => Some("array".to_string()),
            Expression::Instantiation(inst) => {
                // Extract the class name from the instantiation
                let class_text = self.get_span_text(&inst.class.span()).to_lowercase();

                // Handle self, static, and parent keywords
                if class_text == "self" || class_text == "static" {
                    // Return the current class name if we're in a class context
                    if let Some(ref current_class) = self.current_class {
                        return Some(current_class.clone());
                    }
                }

                // For regular class names, try to extract the identifier
                if let Expression::Identifier(ident) = &*inst.class {
                    Some(self.get_span_text(&ident.span()).to_string())
                } else {
                    // For other cases (like parent, variables, etc.), return the text as-is
                    // but only if it looks like a class name (starts with uppercase or special keyword)
                    let text = self.get_span_text(&inst.class.span());
                    Some(text.to_string())
                }
            }
            Expression::Closure(_) | Expression::ArrowFunction(_) => Some("Closure".to_string()),
            _ => None, // Complex expressions - can't easily infer
        }
    }

    /// Check if two types are compatible
    fn types_compatible(&self, expected: &str, actual: &str, current_class: Option<&str>) -> bool {
        if expected == actual {
            return true;
        }

        // Handle union types - expected is a union (e.g., "int|null|string")
        if expected.contains('|') {
            let expected_types: Vec<&str> = expected.split('|').map(|s| s.trim()).collect();
            // If actual matches any member of the union, it's compatible
            for expected_type in expected_types {
                if self.types_compatible(expected_type, actual, current_class) {
                    return true;
                }
            }
            return false;
        }

        // Handle union types - actual is a union
        // All members of actual union must be compatible with expected
        if actual.contains('|') {
            let actual_types: Vec<&str> = actual.split('|').map(|s| s.trim()).collect();
            return actual_types.iter().all(|actual_type| {
                self.types_compatible(expected, actual_type, current_class)
            });
        }

        // Closure is callable in PHP
        if expected == "callable" && actual == "closure" {
            return true;
        }

        // Handle self/static keywords - they refer to the current class
        if let Some(class_name) = current_class {
            let class_lower = class_name.to_lowercase();

            // Expected is self/static, actual is the class name
            if (expected == "self" || expected == "static") && actual == class_lower {
                return true;
            }

            // Actual is self/static, expected is the class name
            if (actual == "self" || actual == "static") && expected == class_lower {
                return true;
            }

            // Both are self/static
            if (expected == "self" || expected == "static") &&
               (actual == "self" || actual == "static") {
                return true;
            }
        }

        // mixed accepts everything
        if expected == "mixed" {
            return true;
        }

        // null is compatible with nullable types
        if actual == "null" && expected.starts_with('?') {
            return true;
        }

        // Non-null value is compatible with nullable type (?string accepts string)
        if expected.starts_with('?') {
            let base_type = &expected[1..];
            if actual == base_type {
                return true;
            }
        }

        // int is compatible with float
        if expected == "float" && actual == "int" {
            return true;
        }

        // Scalar types
        if expected == "scalar" && matches!(actual, "int" | "float" | "string" | "bool") {
            return true;
        }

        // object type accepts any class
        if expected == "object" {
            return true;
        }

        // iterable accepts arrays
        if expected == "iterable" && actual == "array" {
            return true;
        }

        false
    }

    /// Check if all code paths in a block return
    fn block_all_paths_return<'a>(&self, block: &Block<'a>) -> bool {
        for stmt in block.statements.iter() {
            if self.statement_returns(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement always returns
    fn statement_returns<'a>(&self, stmt: &Statement<'a>) -> bool {
        match stmt {
            Statement::Return(_) => true,
            // Throw is an expression in mago_syntax
            Statement::Expression(expr_stmt) => {
                matches!(&expr_stmt.expression, Expression::Throw(_))
            }
            Statement::If(if_stmt) => self.if_all_branches_return(&if_stmt.body),
            _ => false,
        }
    }

    /// Check if all branches of an if statement return
    fn if_all_branches_return<'a>(&self, body: &IfBody<'a>) -> bool {
        match body {
            IfBody::Statement(stmt_body) => {
                // Must have else clause
                if stmt_body.else_clause.is_none() {
                    return false;
                }

                // Check if branch returns
                if !self.statement_returns(stmt_body.statement) {
                    return false;
                }

                // Check all elseif branches
                for else_if in stmt_body.else_if_clauses.iter() {
                    if !self.statement_returns(else_if.statement) {
                        return false;
                    }
                }

                // Check else branch
                if let Some(else_clause) = &stmt_body.else_clause {
                    return self.statement_returns(else_clause.statement);
                }

                false
            }
            IfBody::ColonDelimited(block) => {
                // Must have else clause
                if block.else_clause.is_none() {
                    return false;
                }

                // Check if block
                let if_returns = block.statements.iter().any(|s| self.statement_returns(s));
                if !if_returns {
                    return false;
                }

                // Check all elseif branches
                for else_if in block.else_if_clauses.iter() {
                    if !else_if.statements.iter().any(|s| self.statement_returns(s)) {
                        return false;
                    }
                }

                // Check else branch
                if let Some(else_clause) = &block.else_clause {
                    return else_clause.statements.iter().any(|s| self.statement_returns(s));
                }

                false
            }
        }
    }

    /// Check if a block contains any yield expressions (generator function)
    fn block_contains_yield<'a>(&self, block: &Block<'a>) -> bool {
        for stmt in block.statements.iter() {
            if self.statement_contains_yield(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement contains yield
    fn statement_contains_yield<'a>(&self, stmt: &Statement<'a>) -> bool {
        match stmt {
            Statement::Expression(expr_stmt) => self.expression_contains_yield(&expr_stmt.expression),
            Statement::Return(ret) => {
                ret.value.as_ref().map_or(false, |v| self.expression_contains_yield(v))
            }
            Statement::Block(block) => self.block_contains_yield(block),
            Statement::If(if_stmt) => {
                if self.expression_contains_yield(&if_stmt.condition) {
                    return true;
                }
                self.if_body_contains_yield(&if_stmt.body)
            }
            Statement::While(while_stmt) => {
                if self.expression_contains_yield(&while_stmt.condition) {
                    return true;
                }
                self.while_body_contains_yield(&while_stmt.body)
            }
            Statement::For(for_stmt) => {
                for expr in for_stmt.initializations.iter() {
                    if self.expression_contains_yield(expr) {
                        return true;
                    }
                }
                for expr in for_stmt.conditions.iter() {
                    if self.expression_contains_yield(expr) {
                        return true;
                    }
                }
                for expr in for_stmt.increments.iter() {
                    if self.expression_contains_yield(expr) {
                        return true;
                    }
                }
                self.for_body_contains_yield(&for_stmt.body)
            }
            Statement::Foreach(foreach) => {
                if self.expression_contains_yield(&foreach.expression) {
                    return true;
                }
                self.foreach_body_contains_yield(&foreach.body)
            }
            Statement::Try(try_stmt) => {
                if self.block_contains_yield(&try_stmt.block) {
                    return true;
                }
                for catch in try_stmt.catch_clauses.iter() {
                    if self.block_contains_yield(&catch.block) {
                        return true;
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    if self.block_contains_yield(&finally.block) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if an expression contains yield
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

    fn if_body_contains_yield<'a>(&self, body: &IfBody<'a>) -> bool {
        match body {
            IfBody::Statement(stmt_body) => {
                if self.statement_contains_yield(stmt_body.statement) {
                    return true;
                }
                for else_if in stmt_body.else_if_clauses.iter() {
                    if self.expression_contains_yield(&else_if.condition) {
                        return true;
                    }
                    if self.statement_contains_yield(else_if.statement) {
                        return true;
                    }
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    if self.statement_contains_yield(else_clause.statement) {
                        return true;
                    }
                }
                false
            }
            IfBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    if self.statement_contains_yield(inner) {
                        return true;
                    }
                }
                for else_if in block.else_if_clauses.iter() {
                    if self.expression_contains_yield(&else_if.condition) {
                        return true;
                    }
                    for inner in else_if.statements.iter() {
                        if self.statement_contains_yield(inner) {
                            return true;
                        }
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        if self.statement_contains_yield(inner) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    fn while_body_contains_yield<'a>(&self, body: &WhileBody<'a>) -> bool {
        match body {
            WhileBody::Statement(stmt) => self.statement_contains_yield(stmt),
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    if self.statement_contains_yield(inner) {
                        return true;
                    }
                }
                false
            }
        }
    }

    fn for_body_contains_yield<'a>(&self, body: &ForBody<'a>) -> bool {
        match body {
            ForBody::Statement(stmt) => self.statement_contains_yield(stmt),
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    if self.statement_contains_yield(inner) {
                        return true;
                    }
                }
                false
            }
        }
    }

    fn foreach_body_contains_yield<'a>(&self, body: &ForeachBody<'a>) -> bool {
        match body {
            ForeachBody::Statement(stmt) => self.statement_contains_yield(stmt),
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    if self.statement_contains_yield(inner) {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Check if a block has any side effects
    /// Side effects include: echo/print, function calls, property assignments,
    /// global variable access, throwing exceptions, etc.
    fn block_has_side_effects<'a>(&self, block: &Block<'a>) -> bool {
        for stmt in block.statements.iter() {
            if self.statement_has_side_effects(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement has side effects
    fn statement_has_side_effects<'a>(&self, stmt: &Statement<'a>) -> bool {
        match stmt {
            // Output statements are side effects
            Statement::Echo(_) => true,

            // Expression statements - check for side effects in the expression
            Statement::Expression(expr_stmt) => {
                self.expression_has_side_effects(&expr_stmt.expression)
            }

            // Control flow with potential side effects
            Statement::If(if_stmt) => {
                // Condition could have side effects
                if self.expression_has_side_effects(&if_stmt.condition) {
                    return true;
                }
                self.if_body_has_side_effects(&if_stmt.body)
            }

            Statement::While(while_stmt) => {
                if self.expression_has_side_effects(&while_stmt.condition) {
                    return true;
                }
                self.while_body_has_side_effects(&while_stmt.body)
            }

            Statement::For(for_stmt) => {
                // For loop initializers, conditions, increments can have side effects
                for init in for_stmt.initializations.iter() {
                    if self.expression_has_side_effects(init) {
                        return true;
                    }
                }
                for cond in for_stmt.conditions.iter() {
                    if self.expression_has_side_effects(cond) {
                        return true;
                    }
                }
                for inc in for_stmt.increments.iter() {
                    if self.expression_has_side_effects(inc) {
                        return true;
                    }
                }
                self.for_body_has_side_effects(&for_stmt.body)
            }

            Statement::Foreach(foreach) => {
                if self.expression_has_side_effects(&foreach.expression) {
                    return true;
                }
                self.foreach_body_has_side_effects(&foreach.body)
            }

            Statement::Try(try_stmt) => {
                for inner in try_stmt.block.statements.iter() {
                    if self.statement_has_side_effects(inner) {
                        return true;
                    }
                }
                for catch in try_stmt.catch_clauses.iter() {
                    for inner in catch.block.statements.iter() {
                        if self.statement_has_side_effects(inner) {
                            return true;
                        }
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    for inner in finally.block.statements.iter() {
                        if self.statement_has_side_effects(inner) {
                            return true;
                        }
                    }
                }
                false
            }

            Statement::Block(block) => self.block_has_side_effects(block),

            // Return statements are not side effects (they're control flow, not observable effects)
            Statement::Return(_) => false,

            // Global/static declarations could be considered side effects
            Statement::Global(_) => true,
            Statement::Static(_) => true,

            // Other statements that are side effects
            Statement::Unset(_) => true,

            _ => false,
        }
    }

    /// Check if an expression has side effects
    fn expression_has_side_effects<'a>(&self, expr: &Expression<'a>) -> bool {
        match expr {
            // Function/method calls are side effects (they could do anything)
            Expression::Call(_) => true,

            // Assignments - only side effect if LHS is external state or RHS has side effects
            Expression::Assignment(assign) => {
                // Check if LHS is property access (mutating object state)
                let lhs_is_external = matches!(
                    assign.lhs,
                    Expression::Access(Access::Property(_)) |
                    Expression::Access(Access::NullSafeProperty(_)) |
                    Expression::Access(Access::StaticProperty(_))
                );
                if lhs_is_external {
                    return true;
                }
                // Check if RHS has side effects
                self.expression_has_side_effects(assign.rhs)
            }

            // Increment/decrement are side effects
            Expression::UnaryPostfix(p) => {
                matches!(p.operator, UnaryPostfixOperator::PostIncrement(_) | UnaryPostfixOperator::PostDecrement(_))
            }
            Expression::UnaryPrefix(p) => {
                matches!(p.operator, UnaryPrefixOperator::PreIncrement(_) | UnaryPrefixOperator::PreDecrement(_))
            }

            // Throw is a side effect
            Expression::Throw(_) => true,

            // Yield is a side effect (generator)
            Expression::Yield(_) => true,

            // Clone could have side effects (__clone method)
            Expression::Clone(_) => true,

            // Instantiation could have side effects (constructor)
            Expression::Instantiation(_) => true,

            // Binary expressions - check both sides
            Expression::Binary(binary) => {
                self.expression_has_side_effects(&binary.lhs) ||
                self.expression_has_side_effects(&binary.rhs)
            }

            // Ternary - check all parts
            Expression::Conditional(cond) => {
                self.expression_has_side_effects(&cond.condition) ||
                cond.then.as_ref().map_or(false, |t| self.expression_has_side_effects(t)) ||
                self.expression_has_side_effects(&cond.r#else)
            }

            // Parenthesized - check inner
            Expression::Parenthesized(p) => self.expression_has_side_effects(&p.expression),

            // Most other expressions are pure (literals, variables, array access, etc.)
            _ => false,
        }
    }

    fn if_body_has_side_effects<'a>(&self, body: &IfBody<'a>) -> bool {
        match body {
            IfBody::Statement(stmt_body) => {
                if self.statement_has_side_effects(stmt_body.statement) {
                    return true;
                }
                for else_if in stmt_body.else_if_clauses.iter() {
                    if self.expression_has_side_effects(&else_if.condition) {
                        return true;
                    }
                    if self.statement_has_side_effects(else_if.statement) {
                        return true;
                    }
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    if self.statement_has_side_effects(else_clause.statement) {
                        return true;
                    }
                }
                false
            }
            IfBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    if self.statement_has_side_effects(inner) {
                        return true;
                    }
                }
                for else_if in block.else_if_clauses.iter() {
                    if self.expression_has_side_effects(&else_if.condition) {
                        return true;
                    }
                    for inner in else_if.statements.iter() {
                        if self.statement_has_side_effects(inner) {
                            return true;
                        }
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        if self.statement_has_side_effects(inner) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    fn while_body_has_side_effects<'a>(&self, body: &WhileBody<'a>) -> bool {
        match body {
            WhileBody::Statement(stmt) => self.statement_has_side_effects(stmt),
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    if self.statement_has_side_effects(inner) {
                        return true;
                    }
                }
                false
            }
        }
    }

    fn for_body_has_side_effects<'a>(&self, body: &ForBody<'a>) -> bool {
        match body {
            ForBody::Statement(stmt) => self.statement_has_side_effects(stmt),
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    if self.statement_has_side_effects(inner) {
                        return true;
                    }
                }
                false
            }
        }
    }

    fn foreach_body_has_side_effects<'a>(&self, body: &ForeachBody<'a>) -> bool {
        match body {
            ForeachBody::Statement(stmt) => self.statement_has_side_effects(stmt),
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    if self.statement_has_side_effects(inner) {
                        return true;
                    }
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_return_type_check_level() {
        let check = ReturnTypeCheck;
        assert_eq!(check.level(), 3);
    }
}
