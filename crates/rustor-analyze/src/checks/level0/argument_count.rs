//! Check for wrong argument counts in function and constructor calls (Level 0)

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::HashMap;

/// Checks for function and constructor calls with wrong number of arguments
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
            class_constructors: HashMap::new(),
            class_names: HashMap::new(), // lowercase -> original
            builtin_classes: ctx.builtin_classes,
            analysis_level: ctx.analysis_level,
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
    class_constructors: HashMap<String, FunctionSignature>, // class name (lowercase) -> constructor signature
    class_names: HashMap<String, String>,                    // class name (lowercase) -> original name
    builtin_classes: &'s [&'static str],
    analysis_level: u8, // Analysis level - "too many args" only reported at level 2+
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
            Statement::Class(class) => {
                let original_name = self.get_span_text(&class.name.span).to_string();
                let class_lower = original_name.to_lowercase();

                // Find the __construct method
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span).to_lowercase();
                        if method_name == "__construct" {
                            let sig = self.analyze_parameters(&method.parameter_list);
                            self.class_constructors.insert(class_lower.clone(), sig);
                            break;
                        }
                    }
                }

                self.class_names.insert(class_lower, original_name);
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
        match expr {
            Expression::Call(Call::Function(call)) => {
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
                        // PHPStan only reports "too many arguments" at level 2+
                        if arg_count > max && self.analysis_level >= 2 {
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
            Expression::Instantiation(inst) => {
                // Get class name
                let class_name = match &*inst.class {
                    Expression::Identifier(ident) => {
                        Some(self.get_span_text(&ident.span()).to_string())
                    }
                    _ => None,
                };

                if let Some(name) = class_name {
                    // Skip built-in classes
                    if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&name)) {
                        return true;
                    }

                    let name_lower = name.to_lowercase();
                    let arg_count = inst
                        .argument_list
                        .as_ref()
                        .map(|al| al.arguments.len())
                        .unwrap_or(0);

                    // Check if we have a constructor signature for this class
                    if let Some(sig) = self.class_constructors.get(&name_lower).cloned() {
                        let class_span = inst.class.span();
                        let display_name = self
                            .class_names
                            .get(&name_lower)
                            .cloned()
                            .unwrap_or(name.clone());

                        if arg_count < sig.min_args {
                            let (line, col) = self.get_line_col(class_span.start.offset as usize);
                            self.issues.push(
                                Issue::error(
                                    "arguments.count",
                                    format!(
                                        "Class {} constructor invoked with {} parameter{}, {} required.",
                                        display_name,
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
                            // PHPStan only reports "too many arguments" at level 2+
                            if arg_count > max && self.analysis_level >= 2 {
                                let (line, col) = self.get_line_col(class_span.start.offset as usize);
                                self.issues.push(
                                    Issue::error(
                                        "arguments.count",
                                        format!(
                                            "Class {} constructor invoked with {} parameter{}, {} required.",
                                            display_name,
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
            }
            _ => {}
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
