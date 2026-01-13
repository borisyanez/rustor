//! Echo with non-string/mixed checking (Level 10)
//!
//! When checkImplicitMixed is enabled (level 10), echo statements must receive
//! values that can be safely converted to string. Mixed types cannot be safely
//! converted to string.
//!
//! Example that fails at level 10:
//! ```php
//! function test($value) { // $value is implicitly mixed
//!     echo $value; // ERROR: cannot convert mixed to string
//! }
//! ```

use crate::checks::{Check, CheckContext};
use crate::issue::{Issue, Severity};
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::{HashSet, HashMap};

/// Check for echo statements with mixed/non-string types
pub struct EchoNonStringCheck;

impl Check for EchoNonStringCheck {
    fn id(&self) -> &'static str {
        "echo.nonString"
    }

    fn description(&self) -> &'static str {
        "Checks that echo parameters can be converted to string (not mixed)"
    }

    fn level(&self) -> u8 {
        10
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = EchoNonStringVisitor {
            source: ctx.source,
            file_path: ctx.file_path,
            mixed_vars: HashSet::new(),
            defined_vars: HashSet::new(),
            class_properties: HashMap::new(),
            issues: Vec::new(),
        };

        visitor.visit_program(program);
        visitor.issues
    }
}

struct EchoNonStringVisitor<'s> {
    source: &'s str,
    file_path: &'s std::path::Path,
    /// Variables that are mixed (explicit or implicit)
    mixed_vars: HashSet<String>,
    /// Variables that have been defined/assigned in current scope
    defined_vars: HashSet<String>,
    /// Class properties (class name -> property names)
    class_properties: HashMap<String, HashSet<String>>,
    issues: Vec<Issue>,
}

impl<'s> EchoNonStringVisitor<'s> {
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

    fn is_mixed_hint(&self, hint: &Hint<'_>) -> bool {
        matches!(hint, Hint::Mixed(_))
    }

    fn visit_program<'a>(&mut self, program: &Program<'a>) {
        // First pass: collect class properties
        self.collect_class_properties(program);

        // Second pass: check echo statements
        for stmt in program.statements.iter() {
            self.visit_statement(stmt);
        }
    }

    fn collect_class_properties<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            if let Statement::Class(class) = stmt {
                let class_name = self.get_span_text(&class.name.span).to_lowercase();
                let mut properties = HashSet::new();

                for member in class.members.iter() {
                    if let ClassLikeMember::Property(prop) = member {
                        for var in prop.variables() {
                            let prop_name = self.get_span_text(&var.span()).trim_start_matches('$');
                            properties.insert(prop_name.to_string());
                        }
                    }
                }

                self.class_properties.insert(class_name, properties);
            }
        }
    }

    fn visit_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                // Save old state
                let old_mixed_vars = self.mixed_vars.clone();
                let old_defined_vars = self.defined_vars.clone();

                // Collect mixed and untyped parameters
                self.mixed_vars.clear();
                self.defined_vars.clear();

                for param in func.parameter_list.parameters.iter() {
                    let param_name = self.get_span_text(&param.variable.span()).trim_start_matches('$').to_string();

                    // Mark parameter as defined
                    self.defined_vars.insert(param_name.clone());

                    if let Some(hint) = &param.hint {
                        if self.is_mixed_hint(hint) {
                            self.mixed_vars.insert(param_name);
                        }
                    } else {
                        // No type hint = implicit mixed
                        self.mixed_vars.insert(param_name);
                    }
                }

                // Visit function body
                for inner in func.body.statements.iter() {
                    self.visit_body_statement(inner);
                }

                // Restore old state
                self.mixed_vars = old_mixed_vars;
                self.defined_vars = old_defined_vars;
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(concrete) = &method.body {
                            // Save old state
                            let old_mixed_vars = self.mixed_vars.clone();
                            let old_defined_vars = self.defined_vars.clone();

                            // Collect mixed and untyped parameters
                            self.mixed_vars.clear();
                            self.defined_vars.clear();

                            for param in method.parameter_list.parameters.iter() {
                                let param_name = self.get_span_text(&param.variable.span()).trim_start_matches('$').to_string();

                                // Mark parameter as defined
                                self.defined_vars.insert(param_name.clone());

                                if let Some(hint) = &param.hint {
                                    if self.is_mixed_hint(hint) {
                                        self.mixed_vars.insert(param_name);
                                    }
                                } else {
                                    // No type hint = implicit mixed
                                    self.mixed_vars.insert(param_name);
                                }
                            }

                            // Visit method body
                            for inner in concrete.statements.iter() {
                                self.visit_body_statement(inner);
                            }

                            // Restore old state
                            self.mixed_vars = old_mixed_vars;
                            self.defined_vars = old_defined_vars;
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

    fn visit_body_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Echo(echo) => {
                // Check each value being echoed
                for expr in echo.values.iter() {
                    self.check_echo_expression(expr);
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.visit_body_statement(inner);
                }
            }
            // For other control flow statements, we don't need to recurse
            // as they're complex and the echo check is simple enough
            _ => {}
        }
    }

    fn check_echo_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            Expression::Variable(var) => {
                let var_name = self.get_span_text(&var.span()).trim_start_matches('$');

                // Check if variable is mixed-typed OR undefined
                if self.mixed_vars.contains(var_name) || !self.defined_vars.contains(var_name) {
                    let (line, col) = self.get_line_col(var.span().start.offset as usize);

                    self.issues.push(Issue {
                        check_id: "echo.nonString".to_string(),
                        severity: Severity::Error,
                        message: "Parameter #1 (mixed) of echo cannot be converted to string".to_string(),
                        file: self.file_path.to_path_buf(),
                        line,
                        column: col,
                        identifier: Some("echo.nonString".to_string()),
                        tip: Some(format!(
                            "Variable ${} has mixed type and cannot be safely converted to string. Add type checking or ensure it has a string type.",
                            var_name
                        )),
                    });
                }
            }
            Expression::Access(access) => {
                // For property accesses, we need to determine if the property exists
                // If it doesn't exist, it returns mixed, so echo.nonString applies
                // This is a simplified check - full type inference would be more accurate
                if let Access::Property(prop_access) = access {
                    // We can't easily determine the object type without full type inference
                    // For now, just skip property access checks
                    // PHPStan catches these via the property.notFound check AND echo.nonString
                    // but that requires type resolution we don't have yet
                    let _ = prop_access; // Silence unused warning
                }
            }
            _ => {
                // Other expression types - for now we only check variables and property accesses
            }
        }
    }
}
