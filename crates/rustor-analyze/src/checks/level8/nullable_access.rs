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
//! TODO: Implement full control flow analysis for null checks
//! Currently detects nullable parameters but doesn't track null checks yet

use crate::checks::{Check, CheckContext};
use crate::issue::{Issue, Severity};
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashMap;

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

                // Collect nullable parameters
                self.nullable_params.clear();
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
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        // Handle different method body types
                        match &method.body {
                            MethodBody::Concrete(concrete) => {
                                // Save old state
                                let old_nullable_params = self.nullable_params.clone();

                                // Collect nullable parameters
                                self.nullable_params.clear();
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
            // TODO: Handle if statements to track null checks
            _ => {}
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

    fn check_nullable_access<'a>(
        &mut self,
        target: &Expression<'a>,
        access_type: &str,
        member_span: &mago_span::Span,
    ) {
        // Check if we're accessing a nullable variable
        if let Expression::Variable(var) = target {
            let var_name = self.get_span_text(&var.span()).trim_start_matches('$');

            // Check if this variable is a nullable parameter
            if let Some(type_name) = self.nullable_params.get(var_name) {
                // Report error (TODO: skip if null-checked)
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
