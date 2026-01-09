//! Check for wrong argument counts in function calls (Level 0)

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::HashMap;

/// Checks for function calls with wrong number of arguments
pub struct ArgumentCountCheck;

impl Check for ArgumentCountCheck {
    fn id(&self) -> &'static str {
        "arguments.count"
    }

    fn description(&self) -> &'static str {
        "Detects function calls with wrong argument count"
    }

    fn level(&self) -> u8 {
        0 // PHPStan checks this at level 0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = ArgumentCountVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            function_signatures: HashMap::new(),
            issues: Vec::new(),
        };

        // First pass: collect function signatures
        visitor.collect_definitions(program);

        // Second pass: check function calls
        visitor.visit_program(program, ctx.source);

        visitor.issues
    }
}

/// Information about function parameters
#[derive(Debug, Clone)]
struct FunctionSignature {
    min_args: usize,
    max_args: Option<usize>, // None means variadic
}

struct ArgumentCountVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    function_signatures: HashMap<String, FunctionSignature>,
    issues: Vec<Issue>,
}

impl<'s> ArgumentCountVisitor<'s> {
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
            Statement::Function(func) => {
                let name = self.get_span_text(&func.name.span).to_lowercase();
                let sig = self.analyze_parameters(&func.parameter_list);
                self.function_signatures.insert(name, sig);
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

    fn analyze_parameters(&self, params: &FunctionLikeParameterList<'_>) -> FunctionSignature {
        let mut min_args = 0;
        let mut has_variadic = false;

        for param in params.parameters.iter() {
            if param.ellipsis.is_some() {
                has_variadic = true;
            } else if param.default_value.is_none() {
                min_args += 1;
            }
        }

        let max_args = if has_variadic {
            None
        } else {
            Some(params.parameters.len())
        };

        FunctionSignature { min_args, max_args }
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

impl<'a, 's> Visitor<'a> for ArgumentCountVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get function name
            let name_span = call.function.span();
            let name = self.get_span_text(&name_span);

            // Skip dynamic calls and namespaced calls
            if name.starts_with('$') || name.contains('\\') {
                return true;
            }

            let name_lower = name.to_lowercase();
            let arg_count = call.argument_list.arguments.len();

            // Check if we have a signature for this function
            if let Some(sig) = self.function_signatures.get(&name_lower) {
                if arg_count < sig.min_args {
                    let (line, col) = self.get_line_col(name_span.start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "arguments.count",
                            format!(
                                "Function {} invoked with {} parameter{}, {} required.",
                                name,
                                arg_count,
                                if arg_count == 1 { "" } else { "s" },
                                sig.min_args
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("arguments.count"),
                    );
                } else if let Some(max) = sig.max_args {
                    if arg_count > max {
                        let (line, col) = self.get_line_col(name_span.start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "arguments.count",
                                format!(
                                    "Function {} invoked with {} parameter{}, {} required.",
                                    name,
                                    arg_count,
                                    if arg_count == 1 { "" } else { "s" },
                                    max
                                ),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("arguments.count"),
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
    fn test_argument_count_check_level() {
        let check = ArgumentCountCheck;
        assert_eq!(check.level(), 0); // Should be level 0 like PHPStan
    }
}
