//! Check for calls to undefined methods on known types (Level 2)

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::{HashMap, HashSet};

/// Checks for method calls on objects where we know the type
pub struct CallMethodsCheck;

impl Check for CallMethodsCheck {
    fn id(&self) -> &'static str {
        "method.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects method calls on known types where method doesn't exist"
    }

    fn level(&self) -> u8 {
        2
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = MethodCallVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_methods: HashMap::new(),
            class_names: HashMap::new(),
            variable_types: HashMap::new(),
            builtin_classes: ctx.builtin_classes,
            issues: Vec::new(),
        };

        // First pass: collect class methods
        visitor.collect_definitions(program);

        // Second pass: check method calls
        visitor.visit_program(program, ctx.source);

        visitor.issues
    }
}

struct MethodCallVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    class_methods: HashMap<String, HashSet<String>>, // class name (lowercase) -> method names
    class_names: HashMap<String, String>,             // class name (lowercase) -> original name
    variable_types: HashMap<String, String>,          // variable name -> class name (original)
    builtin_classes: &'s [&'static str],
    issues: Vec<Issue>,
}

impl<'s> MethodCallVisitor<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_from_stmt(stmt);
        }
    }

    fn collect_from_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                let original_name = self.get_span_text(&class.name.span).to_string();
                let class_lower = original_name.to_lowercase();
                let mut methods = HashSet::new();

                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span).to_lowercase();
                        methods.insert(method_name);
                    }
                }

                self.class_names.insert(class_lower.clone(), original_name);
                self.class_methods.insert(class_lower, methods);
            }
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
            _ => {}
        }
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

    /// Extract class name from an instantiation expression
    fn get_instantiation_class<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Instantiation(inst) => match &*inst.class {
                Expression::Identifier(ident) => {
                    Some(self.get_span_text(&ident.span()).to_string())
                }
                _ => None,
            },
            _ => None,
        }
    }
}

impl<'a, 's> Visitor<'a> for MethodCallVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        // Track variable assignments: $obj = new ClassName()
        if let Statement::Expression(expr_stmt) = stmt {
            if let Expression::Assignment(assign) = &expr_stmt.expression {
                // Check if left is a variable and right is an instantiation
                if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                    let var_name = self.get_span_text(&var.span).to_string();
                    if let Some(class_name) = self.get_instantiation_class(assign.rhs) {
                        // Store original class name (type tracking preserves case)
                        self.variable_types.insert(var_name, class_name);
                    }
                }
            }
        }
        true
    }

    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Check for $obj->method() calls
        if let Expression::Call(Call::Method(call)) = expr {
            // Get method name
            let method_info = match &call.method {
                ClassLikeMemberSelector::Identifier(ident) => {
                    Some((self.get_span_text(&ident.span).to_string(), ident.span))
                }
                _ => None,
            };

            if let Some((method, method_span)) = method_info {
                let method_lower = method.to_lowercase();

                // Case 1: (new ClassName())->method()
                if let Some(class_name) = self.get_instantiation_class(&call.object) {
                    // Skip built-in classes
                    if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&class_name)) {
                        return true;
                    }

                    let class_lower = class_name.to_lowercase();
                    if let Some(methods) = self.class_methods.get(&class_lower) {
                        if !methods.contains(&method_lower) {
                            let (line, col) = self.get_line_col(method_span.start.offset as usize);
                            self.issues.push(
                                Issue::error(
                                    "method.notFound",
                                    format!(
                                        "Call to an undefined method {}::{}().",
                                        class_name, method
                                    ),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("method.notFound"),
                            );
                        }
                    }
                }
                // Case 2: $obj->method() where $obj was assigned from new ClassName()
                else if let Expression::Variable(Variable::Direct(var)) = &*call.object {
                    let var_name = self.get_span_text(&var.span).to_string();
                    if let Some(class_name) = self.variable_types.get(&var_name) {
                        let class_lower = class_name.to_lowercase();
                        // Skip built-in classes
                        if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&class_name)) {
                            return true;
                        }

                        if let Some(methods) = self.class_methods.get(&class_lower) {
                            if !methods.contains(&method_lower) {
                                let (line, col) = self.get_line_col(method_span.start.offset as usize);
                                self.issues.push(
                                    Issue::error(
                                        "method.notFound",
                                        format!(
                                            "Call to an undefined method {}::{}().",
                                            class_name, method
                                        ),
                                        self.file_path.clone(),
                                        line,
                                        col,
                                    )
                                    .with_identifier("method.notFound"),
                                );
                            }
                        }
                    }
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_check_level() {
        let check = CallMethodsCheck;
        assert_eq!(check.level(), 2);
    }
}
