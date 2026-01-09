//! Missing typehints detection (Level 6)
//!
//! Detects:
//! - Properties without type declarations
//! - Function/method parameters without type hints
//! - Functions/methods without return type hints

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::path::PathBuf;

/// Check for missing typehints
pub struct MissingTypehintCheck;

impl Check for MissingTypehintCheck {
    fn id(&self) -> &'static str {
        "missingType.parameter"
    }

    fn description(&self) -> &'static str {
        "Detects missing type declarations on properties, parameters, and return types"
    }

    fn level(&self) -> u8 {
        6
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = MissingTypehintVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };

        visitor.analyze_program(program);
        visitor.issues
    }
}

struct MissingTypehintVisitor<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

impl<'s> MissingTypehintVisitor<'s> {
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
                self.check_function(&func_name, &func.parameter_list, &func.return_type_hint, func.span());
            }
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();

                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Property(prop) => {
                            self.check_property(&class_name, prop);
                        }
                        ClassLikeMember::Method(method) => {
                            let method_name = self.get_span_text(&method.name.span).to_string();
                            let full_name = format!("{}::{}", class_name, method_name);

                            // Skip magic methods - they have implicit types
                            if method_name.starts_with("__") {
                                continue;
                            }

                            self.check_method(&full_name, &method.parameter_list, &method.return_type_hint, method.span());
                        }
                        _ => {}
                    }
                }
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
            _ => {}
        }
    }

    fn check_function<'a>(
        &mut self,
        func_name: &str,
        params: &FunctionLikeParameterList<'a>,
        return_type: &Option<FunctionLikeReturnTypeHint<'a>>,
        span: mago_span::Span,
    ) {
        // Check parameters
        for param in params.parameters.iter() {
            if param.hint.is_none() {
                let param_name = self.get_span_text(&param.variable.span);
                let (line, col) = self.get_line_col(span.start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "missingType.parameter",
                        format!(
                            "Function {}() has parameter {} with no type specified.",
                            func_name, param_name
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("missingType.parameter"),
                );
            } else if let Some(hint) = &param.hint {
                // Check for plain array type without value type (missingType.iterableValue)
                if self.is_plain_iterable_type(hint) {
                    let param_name = self.get_span_text(&param.variable.span);
                    let (line, col) = self.get_line_col(hint.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "missingType.iterableValue",
                            format!(
                                "Function {}() has parameter {} with no value type specified in iterable type array.",
                                func_name, param_name
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("missingType.iterableValue"),
                    );
                }
            }
        }

        // Check return type
        if return_type.is_none() {
            let (line, col) = self.get_line_col(span.start.offset as usize);
            self.issues.push(
                Issue::error(
                    "missingType.return",
                    format!("Function {}() has no return type specified.", func_name),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("missingType.return"),
            );
        }
    }

    fn check_method<'a>(
        &mut self,
        method_name: &str,
        params: &FunctionLikeParameterList<'a>,
        return_type: &Option<FunctionLikeReturnTypeHint<'a>>,
        span: mago_span::Span,
    ) {
        // Check parameters
        for param in params.parameters.iter() {
            if param.hint.is_none() {
                let param_name = self.get_span_text(&param.variable.span);
                let (line, col) = self.get_line_col(span.start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "missingType.parameter",
                        format!(
                            "Method {}() has parameter {} with no type specified.",
                            method_name, param_name
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("missingType.parameter"),
                );
            } else if let Some(hint) = &param.hint {
                // Check for plain array type without value type (missingType.iterableValue)
                if self.is_plain_iterable_type(hint) {
                    let param_name = self.get_span_text(&param.variable.span);
                    let (line, col) = self.get_line_col(hint.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "missingType.iterableValue",
                            format!(
                                "Method {}() has parameter {} with no value type specified in iterable type array.",
                                method_name, param_name
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("missingType.iterableValue"),
                    );
                }
            }
        }

        // Check return type
        if return_type.is_none() {
            let (line, col) = self.get_line_col(span.start.offset as usize);
            self.issues.push(
                Issue::error(
                    "missingType.return",
                    format!("Method {}() has no return type specified.", method_name),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("missingType.return"),
            );
        }
    }

    /// Check if a type hint is a plain array/iterable without value type specification
    fn is_plain_iterable_type(&self, hint: &Hint<'_>) -> bool {
        match hint {
            Hint::Array(_) => true,
            Hint::Iterable(_) => true,
            Hint::Nullable(nullable) => self.is_plain_iterable_type(&nullable.hint),
            Hint::Parenthesized(p) => self.is_plain_iterable_type(&p.hint),
            _ => false,
        }
    }

    fn check_property<'a>(&mut self, class_name: &str, prop: &Property<'a>) {
        // Check if property has a type
        if prop.hint().is_none() {
            for var in prop.variables() {
                let prop_name = self.get_span_text(&var.span);
                let (line, col) = self.get_line_col(prop.span().start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "missingType.property",
                        format!(
                            "Property {}::{} has no type specified.",
                            class_name, prop_name
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("missingType.property"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_typehint_check_level() {
        let check = MissingTypehintCheck;
        assert_eq!(check.level(), 6);
    }
}
