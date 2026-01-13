//! Implicit mixed type checking (Level 10)
//!
//! When checkImplicitMixed is enabled (level 10), missing type declarations
//! are treated as implicit `mixed` and the same restrictions from level 9 apply.
//!
//! Example that fails at level 10:
//! ```php
//! function qux($value) { // No type = implicit mixed
//!     strlen($value); // ERROR: can't pass implicit mixed to string
//! }
//! ```

use crate::checks::{Check, CheckContext};
use crate::issue::{Issue, Severity};
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::{HashMap, HashSet};

/// Check for invalid operations on implicit mixed types
pub struct ImplicitMixedCheck;

impl Check for ImplicitMixedCheck {
    fn id(&self) -> &'static str {
        "mixed.implicitUsage"
    }

    fn description(&self) -> &'static str {
        "Checks that implicit mixed types (missing typehints) are only passed to mixed parameters"
    }

    fn level(&self) -> u8 {
        10
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = ImplicitMixedVisitor {
            source: ctx.source,
            file_path: ctx.file_path,
            builtin_functions: ctx.builtin_functions,
            function_params: HashMap::new(),
            untyped_vars: HashSet::new(),
            issues: Vec::new(),
        };

        // First pass: collect function parameter types
        visitor.collect_function_signatures(program);

        // Second pass: check untyped variable usage
        visitor.analyze_program(program);

        visitor.issues
    }
}

struct ImplicitMixedVisitor<'s> {
    source: &'s str,
    file_path: &'s std::path::Path,
    builtin_functions: &'s [&'static str],
    /// Function name (lowercase) -> parameter index -> type (Some = typed, None = untyped/mixed)
    function_params: HashMap<String, Vec<Option<String>>>,
    /// Variable names that have no type hint (implicit mixed) in current scope
    untyped_vars: HashSet<String>,
    issues: Vec<Issue>,
}

impl<'s> ImplicitMixedVisitor<'s> {
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

    fn extract_type_name<'a>(&self, hint: &Hint<'a>) -> Option<String> {
        match hint {
            Hint::Identifier(ident) => Some(self.get_span_text(&ident.span()).to_string()),
            Hint::String(_) => Some("string".to_string()),
            Hint::Integer(_) => Some("int".to_string()),
            Hint::Float(_) => Some("float".to_string()),
            Hint::Bool(_) => Some("bool".to_string()),
            Hint::Array(_) => Some("array".to_string()),
            Hint::Callable(_) => Some("callable".to_string()),
            Hint::Iterable(_) => Some("iterable".to_string()),
            Hint::Object(_) => Some("object".to_string()),
            Hint::Void(_) => Some("void".to_string()),
            Hint::Never(_) => Some("never".to_string()),
            Hint::Null(_) => Some("null".to_string()),
            Hint::True(_) => Some("true".to_string()),
            Hint::False(_) => Some("false".to_string()),
            Hint::Mixed(_) => Some("mixed".to_string()),
            Hint::Nullable(n) => self.extract_type_name(&n.hint),
            Hint::Union(u) => self.extract_type_name(&u.left),
            Hint::Parenthesized(p) => self.extract_type_name(&p.hint),
            _ => None,
        }
    }

    fn collect_function_signatures<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_signatures_from_stmt(stmt);
        }
    }

    fn collect_signatures_from_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                let func_name = self.get_span_text(&func.name.span).to_lowercase();
                let mut param_types = Vec::new();

                for param in func.parameter_list.parameters.iter() {
                    if let Some(hint) = &param.hint {
                        param_types.push(self.extract_type_name(hint));
                    } else {
                        param_types.push(None); // No type hint
                    }
                }

                self.function_params.insert(func_name, param_types);
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span).to_lowercase();
                        let mut param_types = Vec::new();

                        for param in method.parameter_list.parameters.iter() {
                            if let Some(hint) = &param.hint {
                                param_types.push(self.extract_type_name(hint));
                            } else {
                                param_types.push(None);
                            }
                        }

                        self.function_params.insert(method_name, param_types);
                    }
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.collect_signatures_from_stmt(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.collect_signatures_from_stmt(inner);
                    }
                }
            },
            _ => {}
        }
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
                let old_untyped_vars = self.untyped_vars.clone();

                // Collect parameters WITHOUT type hints (implicit mixed)
                self.untyped_vars.clear();
                for param in func.parameter_list.parameters.iter() {
                    if param.hint.is_none() {
                        let param_name = self.get_span_text(&param.variable.span()).trim_start_matches('$');
                        self.untyped_vars.insert(param_name.to_string());
                    }
                }

                // Visit function body
                for inner in func.body.statements.iter() {
                    self.visit_body_statement(inner);
                }

                // Restore old state
                self.untyped_vars = old_untyped_vars;
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        match &method.body {
                            MethodBody::Concrete(concrete) => {
                                // Save old state
                                let old_untyped_vars = self.untyped_vars.clone();

                                // Collect parameters WITHOUT type hints
                                self.untyped_vars.clear();
                                for param in method.parameter_list.parameters.iter() {
                                    if param.hint.is_none() {
                                        let param_name = self.get_span_text(&param.variable.span()).trim_start_matches('$');
                                        self.untyped_vars.insert(param_name.to_string());
                                    }
                                }

                                // Visit method body
                                for inner in concrete.statements.iter() {
                                    self.visit_body_statement(inner);
                                }

                                // Restore old state
                                self.untyped_vars = old_untyped_vars;
                            }
                            MethodBody::Abstract(_) => {}
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
            _ => {}
        }
    }

    fn visit_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            Expression::Call(call) => {
                match call {
                    Call::Function(func_call) => {
                        self.check_function_call(func_call);
                    }
                    Call::Method(method_call) => {
                        // Visit the object
                        self.visit_expression(&method_call.object);
                        // Check method arguments
                        self.check_arguments(&method_call.method.span(), &method_call.argument_list);
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

    fn check_function_call<'a>(&mut self, call: &FunctionCall<'a>) {
        // Get function name
        let func_name = match &call.function {
            Expression::Identifier(ident) => {
                self.get_span_text(&ident.span()).to_lowercase()
            }
            _ => return,
        };

        // Check if it's a builtin function
        if self.builtin_functions.contains(&func_name.as_str()) {
            // Builtin functions typically don't accept untyped
            for (i, arg) in call.argument_list.arguments.iter().enumerate() {
                if let Argument::Positional(positional) = arg {
                    self.check_untyped_argument(&positional.value, &func_name, i, call.function.span().start.offset as usize);
                }
            }
            return;
        }

        // Check user-defined functions
        self.check_arguments(&call.function.span(), &call.argument_list);
    }

    fn check_arguments<'a>(&mut self, func_span: &mago_span::Span, args: &ArgumentList<'a>) {
        let func_name = self.get_span_text(func_span).to_lowercase();

        if let Some(param_types) = self.function_params.get(&func_name).cloned() {
            for (i, arg) in args.arguments.iter().enumerate() {
                if let Argument::Positional(positional) = arg {
                    if let Some(Some(param_type)) = param_types.get(i) {
                        // Parameter has a type - check if we're passing untyped to typed
                        if param_type != "mixed" {
                            self.check_untyped_argument(&positional.value, &func_name, i, func_span.start.offset as usize);
                        }
                    }
                }
            }
        }
    }

    fn check_untyped_argument<'a>(&mut self, arg_expr: &Expression<'a>, func_name: &str, arg_index: usize, offset: usize) {
        // Check if the argument is an untyped variable
        if let Expression::Variable(var) = arg_expr {
            let var_name = self.get_span_text(&var.span()).trim_start_matches('$');

            if self.untyped_vars.contains(var_name) {
                let (line, col) = self.get_line_col(offset);

                // Get the parameter type for better error message
                let unknown = "unknown".to_string();
                let param_type = self.function_params
                    .get(func_name)
                    .and_then(|params| params.get(arg_index))
                    .and_then(|t| t.as_ref())
                    .unwrap_or(&unknown);

                self.issues.push(Issue {
                    check_id: "mixed.implicitUsage".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Cannot pass implicit mixed variable ${} to parameter {} of {} (expects {})",
                        var_name,
                        arg_index + 1,
                        func_name,
                        param_type
                    ),
                    file: self.file_path.to_path_buf(),
                    line,
                    column: col,
                    identifier: Some("argument.implicitMixedToTyped".to_string()),
                    tip: Some(format!(
                        "Parameter {} of {} expects {}, but ${} has no type declaration (implicit mixed). Add a type hint to ${} or change the function parameter to accept mixed.",
                        arg_index + 1,
                        func_name,
                        param_type,
                        var_name,
                        var_name
                    )),
                });
            }
        }
    }
}
