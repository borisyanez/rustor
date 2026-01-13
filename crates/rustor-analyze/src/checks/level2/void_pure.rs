//! Pure void function checking (Level 2)
//!
//! Detects functions that return void but have no side effects.
//!
//! Example that fails:
//! ```php
//! function noSideEffects(): void {
//!     return; // ERROR: void function with no side effects
//! }
//! ```

use crate::checks::{Check, CheckContext};
use crate::issue::{Issue, Severity};
use mago_span::HasSpan;
use mago_syntax::ast::*;

/// Check for void functions with no side effects
pub struct VoidPureCheck;

impl Check for VoidPureCheck {
    fn id(&self) -> &'static str {
        "void.pure"
    }

    fn description(&self) -> &'static str {
        "Checks for void functions that have no side effects"
    }

    fn level(&self) -> u8 {
        2
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = VoidPureVisitor {
            source: ctx.source,
            file_path: ctx.file_path,
            issues: Vec::new(),
        };

        visitor.visit_program(program);
        visitor.issues
    }
}

struct VoidPureVisitor<'s> {
    source: &'s str,
    file_path: &'s std::path::Path,
    issues: Vec<Issue>,
}

impl<'s> VoidPureVisitor<'s> {
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

    fn is_void_return_type(&self, hint: &Hint<'_>) -> bool {
        matches!(hint, Hint::Void(_))
    }

    fn visit_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.visit_statement(stmt);
        }
    }

    fn visit_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                // Check if function returns void
                if let Some(return_hint) = &func.return_type_hint {
                    if self.is_void_return_type(&return_hint.hint) {
                        // Skip empty functions (common placeholder pattern)
                        if func.body.statements.is_empty() {
                            return;
                        }

                        // Check if function has side effects
                        let mut has_side_effects = false;
                        for stmt in func.body.statements.iter() {
                            if self.statement_has_side_effects(stmt) {
                                has_side_effects = true;
                                break;
                            }
                        }

                        if !has_side_effects {
                            let func_name = self.get_span_text(&func.name.span);
                            let (line, col) = self.get_line_col(func.name.span.start.offset as usize);

                            self.issues.push(Issue {
                                check_id: "void.pure".to_string(),
                                severity: Severity::Error,
                                message: format!(
                                    "Function {}() returns void but does not have any side effects",
                                    func_name
                                ),
                                file: self.file_path.to_path_buf(),
                                line,
                                column: col,
                                identifier: Some("void.pure".to_string()),
                                tip: Some(
                                    "Either change the return type or add side effects (assignments, function calls, echo, etc.)".to_string()
                                ),
                            });
                        }
                    }
                }
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(concrete) = &method.body {
                            // Check if method returns void
                            if let Some(return_hint) = &method.return_type_hint {
                                if self.is_void_return_type(&return_hint.hint) {
                                    // Skip empty methods (common placeholder pattern)
                                    if concrete.statements.is_empty() {
                                        continue;
                                    }

                                    // Check if method has side effects
                                    let mut has_side_effects = false;
                                    for stmt in concrete.statements.iter() {
                                        if self.statement_has_side_effects(stmt) {
                                            has_side_effects = true;
                                            break;
                                        }
                                    }

                                    if !has_side_effects {
                                        let method_name = self.get_span_text(&method.name.span);
                                        let (line, col) = self.get_line_col(method.name.span.start.offset as usize);

                                        self.issues.push(Issue {
                                            check_id: "void.pure".to_string(),
                                            severity: Severity::Error,
                                            message: format!(
                                                "Method {}() returns void but does not have any side effects",
                                                method_name
                                            ),
                                            file: self.file_path.to_path_buf(),
                                            line,
                                            column: col,
                                            identifier: Some("void.pure".to_string()),
                                            tip: Some(
                                                "Either change the return type or add side effects (assignments, function calls, echo, etc.)".to_string()
                                            ),
                                        });
                                    }
                                }
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

    fn statement_has_side_effects<'a>(&self, stmt: &Statement<'a>) -> bool {
        match stmt {
            // Empty return has no side effects
            Statement::Return(ret) if ret.value.is_none() => false,

            // Any other return has side effects (evaluating the expression)
            Statement::Return(_) => true,

            // Echo is a side effect
            Statement::Echo(_) => true,

            // Expression statements - check if they're side-effecting
            Statement::Expression(expr_stmt) => {
                self.expression_has_side_effects(&expr_stmt.expression)
            }

            // Control flow statements
            Statement::If(_) | Statement::While(_) | Statement::For(_) | Statement::Foreach(_)
            | Statement::Switch(_) | Statement::Try(_) => true,

            // Block - recursively check
            Statement::Block(block) => {
                for stmt in block.statements.iter() {
                    if self.statement_has_side_effects(stmt) {
                        return true;
                    }
                }
                false
            }

            // Most other statements are side effects
            _ => true,
        }
    }

    fn expression_has_side_effects<'a>(&self, expr: &Expression<'a>) -> bool {
        match expr {
            // Assignments are side effects
            Expression::Assignment(_) => true,

            // Function/method calls are side effects
            Expression::Call(_) => true,

            // Increment/decrement are side effects
            Expression::UnaryPrefix(_) | Expression::UnaryPostfix(_) => {
                // All unary prefix/postfix operations are side effects (++, --, etc.)
                true
            }

            // Literals and simple operations have no side effects
            Expression::Literal(_) | Expression::Variable(_) | Expression::Identifier(_) => false,

            // Binary operations - check operands
            Expression::Binary(bin) => {
                self.expression_has_side_effects(&bin.lhs) || self.expression_has_side_effects(&bin.rhs)
            }

            // Default to no side effects for other cases (access, literals, etc.)
            _ => false,
        }
    }
}
