//! Nullable type access checking (Level 8)
//!
//! When checkNullables is enabled (level 8+), accessing methods/properties
//! on nullable types without null checks is reported as an error.
//!
//! Examples that fail at level 8:
//! ```php
//! function bar(?User $user) {
//!     echo $user->name; // ERROR: Cannot access property on User|null
//!     $user->getName(); // ERROR: Cannot call method on User|null
//! }
//! ```
//!
//! Supports basic control flow analysis:
//! - Early return pattern: if ($var === null) { return; }
//! - Nullsafe operator: $var?->method()
//!
//! TODO: Implement advanced control flow analysis for:
//! - Non-early-return if blocks: if ($var !== null) { ... }
//! - Complex boolean expressions: if ($a && $b) ...
//! - Variable reassignment tracking

use crate::checks::{Check, CheckContext};
use crate::issue::{Issue, Severity};
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::{HashMap, HashSet};

/// Check for method/property access on nullable types
pub struct NullableAccessCheck;

impl Check for NullableAccessCheck {
    fn id(&self) -> &'static str {
        "nullable.access"
    }

    fn description(&self) -> &'static str {
        "Checks for accessing methods/properties on nullable types without null checks"
    }

    fn level(&self) -> u8 {
        8
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = NullableAccessVisitor {
            source: ctx.source,
            file_path: ctx.file_path,
            nullable_params: HashMap::new(),
            null_checked_vars: HashSet::new(),
            issues: Vec::new(),
        };

        visitor.analyze_program(program);
        visitor.issues
    }
}

struct NullableAccessVisitor<'s> {
    source: &'s str,
    file_path: &'s std::path::Path,
    /// Parameter name -> type name (for nullable parameters)
    nullable_params: HashMap<String, String>,
    /// Variables that have been null-checked (narrowed to non-null)
    null_checked_vars: HashSet<String>,
    issues: Vec<Issue>,
}

impl<'s> NullableAccessVisitor<'s> {
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
            self.visit_statement(stmt);
        }
    }

    fn visit_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                // Save old state
                let old_nullable_params = self.nullable_params.clone();
                let old_null_checked = self.null_checked_vars.clone();

                // Collect nullable parameters
                self.nullable_params.clear();
                self.null_checked_vars.clear();
                for param in func.parameter_list.parameters.iter() {
                    let param_name = self.get_span_text(&param.variable.span()).trim_start_matches('$');
                    if let Some(hint) = &param.hint {
                        if let Some(type_name) = self.extract_nullable_type(hint) {
                            self.nullable_params.insert(param_name.to_string(), type_name);
                        }
                    }
                }

                // Visit function body
                for inner in func.body.statements.iter() {
                    self.visit_body_statement(inner);
                }

                // Restore old state
                self.nullable_params = old_nullable_params;
                self.null_checked_vars = old_null_checked;
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        // Handle different method body types
                        match &method.body {
                            MethodBody::Concrete(concrete) => {
                                // Save old state
                                let old_nullable_params = self.nullable_params.clone();
                                let old_null_checked = self.null_checked_vars.clone();

                                // Collect nullable parameters
                                self.nullable_params.clear();
                                self.null_checked_vars.clear();
                                for param in method.parameter_list.parameters.iter() {
                                    let param_name = self.get_span_text(&param.variable.span()).trim_start_matches('$');
                                    if let Some(hint) = &param.hint {
                                        if let Some(type_name) = self.extract_nullable_type(hint) {
                                            self.nullable_params.insert(param_name.to_string(), type_name);
                                        }
                                    }
                                }

                                // Visit method body
                                for inner in concrete.statements.iter() {
                                    self.visit_body_statement(inner);
                                }

                                // Restore old state
                                self.nullable_params = old_nullable_params;
                                self.null_checked_vars = old_null_checked;
                            }
                            MethodBody::Abstract(_) => {
                                // No body to analyze
                            }
                        }
                    }
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.visit_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.visit_statement(inner);
                    }
                }
            },
            _ => {}
        }
    }

    /// Extract type name from a nullable type hint (?Type)
    fn extract_nullable_type<'a>(&self, hint: &Hint<'a>) -> Option<String> {
        match hint {
            Hint::Nullable(nullable) => {
                // ?Type format
                match &*nullable.hint {
                    Hint::Identifier(ident) => Some(self.get_span_text(&ident.span()).to_string()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn visit_body_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.visit_expression(&expr_stmt.expression);
            }
            Statement::Echo(echo) => {
                for expr in echo.values.iter() {
                    self.visit_expression(expr);
                }
            }
            Statement::Return(ret) => {
                if let Some(expr) = &ret.value {
                    self.visit_expression(expr);
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.visit_body_statement(inner);
                }
            }
            Statement::If(if_stmt) => {
                // Pattern 1: if ($var === null) { return; } - variable is non-null AFTER the if
                if let Some(var_name) = self.extract_null_check_with_early_return(if_stmt) {
                    self.null_checked_vars.insert(var_name.clone());

                    // Visit the if statement
                    self.visit_expression(&if_stmt.condition);
                    self.visit_if_body(&if_stmt.body);
                }
                // Pattern 2: if ($var !== null) { ... } - variable is non-null INSIDE the if
                else if let Some(var_name) = self.extract_not_null_check(&if_stmt.condition) {
                    // Save current null-checked vars
                    let saved_null_checked = self.null_checked_vars.clone();

                    // Add variable to null-checked set for inside the if block
                    self.null_checked_vars.insert(var_name);

                    // Visit the if statement
                    self.visit_expression(&if_stmt.condition);
                    self.visit_if_body(&if_stmt.body);

                    // Restore original null-checked vars (variable only narrowed inside if)
                    self.null_checked_vars = saved_null_checked;
                } else {
                    // No null check pattern - just visit normally
                    self.visit_expression(&if_stmt.condition);
                    self.visit_if_body(&if_stmt.body);
                }
            }
            _ => {}
        }
    }

    fn visit_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.visit_body_statement(&stmt_body.statement);

                // Visit elseif clauses
                for elseif in stmt_body.else_if_clauses.iter() {
                    self.visit_expression(&elseif.condition);
                    self.visit_body_statement(&elseif.statement);
                }

                // Visit else clause
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.visit_body_statement(&else_clause.statement);
                }
            }
            IfBody::ColonDelimited(colon_body) => {
                for inner in colon_body.statements.iter() {
                    self.visit_body_statement(inner);
                }

                // Visit elseif clauses
                for elseif in colon_body.else_if_clauses.iter() {
                    self.visit_expression(&elseif.condition);
                    for inner in elseif.statements.iter() {
                        self.visit_body_statement(inner);
                    }
                }

                // Visit else clause
                if let Some(else_clause) = &colon_body.else_clause {
                    for inner in else_clause.statements.iter() {
                        self.visit_body_statement(inner);
                    }
                }
            }
        }
    }

    fn visit_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // Check property/method access patterns
            Expression::Access(access) => {
                match access {
                    Access::Property(prop) => {
                        self.check_nullable_access(&prop.object, "property", &prop.property.span());
                        self.visit_expression(&prop.object);
                    }
                    Access::NullSafeProperty(prop) => {
                        // ?-> doesn't need null check
                        self.visit_expression(&prop.object);
                    }
                    _ => {}
                }
            }
            // Check method calls
            Expression::Call(call) => {
                match call {
                    Call::Method(method) => {
                        self.check_nullable_access(&method.object, "method", &method.method.span());
                        self.visit_expression(&method.object);
                    }
                    Call::NullSafeMethod(_) => {
                        // ?-> doesn't need null check
                    }
                    _ => {}
                }
            }
            Expression::Binary(bin) => {
                self.visit_expression(&bin.lhs);
                self.visit_expression(&bin.rhs);
            }
            Expression::Assignment(assign) => {
                self.visit_expression(&assign.rhs);
            }
            _ => {}
        }
    }

    /// Extract variable name from not-null check
    /// Detects: if ($var !== null) or if ($var != null)
    fn extract_not_null_check<'a>(&self, condition: &Expression<'a>) -> Option<String> {
        if let Expression::Binary(bin) = condition {
            // Check for !== or != with null
            let is_not_equal_op = matches!(
                bin.operator,
                BinaryOperator::NotIdentical(_) | BinaryOperator::NotEqual(_)
            );

            if !is_not_equal_op {
                return None;
            }

            // Check if one side is a variable and the other is null
            match (&*bin.lhs, &*bin.rhs) {
                (Expression::Variable(var), Expression::Literal(Literal::Null(_))) => {
                    Some(self.get_span_text(&var.span()).trim_start_matches('$').to_string())
                }
                (Expression::Literal(Literal::Null(_)), Expression::Variable(var)) => {
                    Some(self.get_span_text(&var.span()).trim_start_matches('$').to_string())
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Extract variable name from null check with early return pattern
    /// Detects: if ($var === null) { return ...; }
    fn extract_null_check_with_early_return<'a>(&self, if_stmt: &If<'a>) -> Option<String> {
        // Check if condition is: $var === null
        if let Expression::Binary(bin) = &if_stmt.condition {
            // Check for === or == with null
            let is_equal_op = matches!(
                bin.operator,
                BinaryOperator::Identical(_) | BinaryOperator::Equal(_)
            );

            if !is_equal_op {
                return None;
            }

            // Check if one side is a variable and the other is null
            let var_name = match (&*bin.lhs, &*bin.rhs) {
                (Expression::Variable(var), Expression::Literal(Literal::Null(_))) => {
                    Some(self.get_span_text(&var.span()).trim_start_matches('$').to_string())
                }
                (Expression::Literal(Literal::Null(_)), Expression::Variable(var)) => {
                    Some(self.get_span_text(&var.span()).trim_start_matches('$').to_string())
                }
                _ => None,
            }?;

            // Check if body contains return statement
            let has_return = match &if_stmt.body {
                IfBody::Statement(stmt_body) => {
                    self.statement_contains_return(&stmt_body.statement)
                }
                IfBody::ColonDelimited(colon_body) => {
                    colon_body.statements.iter().any(|stmt| {
                        matches!(stmt, Statement::Return(_))
                    })
                }
            };

            if has_return {
                return Some(var_name);
            }
        }

        None
    }

    /// Check if a statement contains a return
    fn statement_contains_return<'a>(&self, stmt: &Statement<'a>) -> bool {
        match stmt {
            Statement::Return(_) => true,
            Statement::Block(block) => {
                block.statements.iter().any(|s| matches!(s, Statement::Return(_)))
            }
            _ => false,
        }
    }

    fn check_nullable_access<'a>(
        &mut self,
        target: &Expression<'a>,
        access_type: &str,
        member_span: &mago_span::Span,
    ) {
        // Check if we're accessing a nullable variable
        if let Expression::Variable(var) = target {
            let var_name = self.get_span_text(&var.span()).trim_start_matches('$');

            // Skip if variable has been null-checked
            if self.null_checked_vars.contains(var_name) {
                return;
            }

            // Check if this variable is a nullable parameter
            if let Some(type_name) = self.nullable_params.get(var_name) {
                // Report error
                let (line, col) = self.get_line_col(target.span().start.offset as usize);
                let member_name = self.get_span_text(member_span);

                let message = format!(
                    "Cannot {} {} on {}|null",
                    if access_type == "property" { "access property" } else { "call method" },
                    member_name,
                    type_name
                );

                self.issues.push(Issue {
                    check_id: self.id().to_string(),
                    severity: Severity::Error,
                    message,
                    file: self.file_path.to_path_buf(),
                    line,
                    column: col,
                    identifier: Some(if access_type == "property" {
                        "property.nonObject".to_string()
                    } else {
                        "method.nonObject".to_string()
                    }),
                    tip: Some(format!("Add a null check: if (${} !== null) {{ ... }}", var_name)),
                });
            }
        }
    }

    fn id(&self) -> &str {
        "nullable.access"
    }
}
