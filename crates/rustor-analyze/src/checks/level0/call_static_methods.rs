//! Check for calls to undefined static methods (Level 0)

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::{HashMap, HashSet};

/// Checks for static method calls like Foo::bar()
pub struct CallStaticMethodsCheck;

impl Check for CallStaticMethodsCheck {
    fn id(&self) -> &'static str {
        "staticMethod.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects calls to undefined static methods"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = StaticMethodCallVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_methods: HashMap::new(),
            builtin_classes: ctx.builtin_classes,
            issues: Vec::new(),
        };

        // First pass: collect class definitions with methods
        visitor.collect_definitions(program);

        // Second pass: check static method calls
        visitor.visit_program(program, ctx.source);

        visitor.issues
    }
}

struct StaticMethodCallVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    class_methods: HashMap<String, HashSet<String>>, // class name -> method names
    builtin_classes: &'s [&'static str],
    issues: Vec<Issue>,
}

impl<'s> StaticMethodCallVisitor<'s> {
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
                let class_name = self.get_span_text(&class.name.span).to_lowercase();
                let mut methods = HashSet::new();

                // Collect all methods (both static and non-static, as static:: can call non-static)
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span).to_lowercase();
                        methods.insert(method_name);
                    }
                }

                self.class_methods.insert(class_name, methods);
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
}

impl<'a, 's> Visitor<'a> for StaticMethodCallVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Check for Class::method() calls
        if let Expression::Call(Call::StaticMethod(call)) = expr {
            // Get class name from the target
            let class_name = match &*call.class {
                Expression::Identifier(ident) => {
                    Some(self.get_span_text(&ident.span()).to_string())
                }
                _ => None,
            };

            // Get method name
            let method_info = match &call.method {
                ClassLikeMemberSelector::Identifier(ident) => {
                    Some((self.get_span_text(&ident.span).to_string(), ident.span))
                }
                _ => None,
            };

            if let (Some(class), Some((method, method_span))) = (class_name, method_info) {
                // Skip self, static, parent
                if class.eq_ignore_ascii_case("self")
                    || class.eq_ignore_ascii_case("static")
                    || class.eq_ignore_ascii_case("parent")
                {
                    return true;
                }

                // Skip built-in classes - they have many methods we don't track
                if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&class)) {
                    return true;
                }

                let class_lower = class.to_lowercase();
                let method_lower = method.to_lowercase();

                // Check if we have this class defined and if it has the method
                if let Some(methods) = self.class_methods.get(&class_lower) {
                    if !methods.contains(&method_lower) {
                        let (line, col) = self.get_line_col(method_span.start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "staticMethod.notFound",
                                format!(
                                    "Call to an undefined static method {}::{}().",
                                    class, method
                                ),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("staticMethod.notFound"),
                        );
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
    fn test_static_method_check_level() {
        let check = CallStaticMethodsCheck;
        assert_eq!(check.level(), 0);
    }
}
