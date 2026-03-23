//! Check for required parameters after optional parameters (Level 0)
//!
//! PHPStan reports when a required parameter follows an optional one.
//! This was deprecated in PHP 8.0.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::path::PathBuf;

pub struct RequiredAfterOptionalCheck;

impl Check for RequiredAfterOptionalCheck {
    fn id(&self) -> &'static str {
        "parameter.requiredAfterOptional"
    }

    fn description(&self) -> &'static str {
        "Detects required parameters after optional parameters"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = RequiredAfterOptionalAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };
        analyzer.analyze_program(program);
        analyzer.issues
    }
}

struct RequiredAfterOptionalAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

impl<'s> RequiredAfterOptionalAnalyzer<'s> {
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
                self.check_parameter_list(&func.parameter_list);
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        self.check_parameter_list(&method.parameter_list);
                        if let MethodBody::Concrete(body) = &method.body {
                            for inner in body.statements.iter() {
                                self.analyze_statement(inner);
                            }
                        }
                    }
                }
            }
            Statement::Interface(iface) => {
                for member in iface.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        self.check_parameter_list(&method.parameter_list);
                    }
                }
            }
            Statement::Trait(tr) => {
                for member in tr.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        self.check_parameter_list(&method.parameter_list);
                        if let MethodBody::Concrete(body) = &method.body {
                            for inner in body.statements.iter() {
                                self.analyze_statement(inner);
                            }
                        }
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
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            _ => {}
        }
    }

    fn check_parameter_list<'a>(&mut self, params: &FunctionLikeParameterList<'a>) {
        let mut last_optional: Option<String> = None;

        for param in params.parameters.iter() {
            // Variadic parameters are always last and implicitly optional — skip
            if param.ellipsis.is_some() {
                continue;
            }

            let param_name = self.get_span_text(&param.variable.span()).to_string();

            if param.default_value.is_some() {
                // This parameter is optional (has a default)
                last_optional = Some(param_name);
            } else {
                // This parameter is required (no default, not variadic)
                if let Some(ref optional_name) = last_optional {
                    let (line, col) = self.get_line_col(param.variable.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "parameter.requiredAfterOptional",
                            format!(
                                "Deprecated in PHP 8.0: Required parameter {} follows optional parameter {}.",
                                param_name,
                                optional_name,
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("parameter.requiredAfterOptional"),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_after_optional_check_level() {
        let check = RequiredAfterOptionalCheck;
        assert_eq!(check.level(), 0);
    }

    #[test]
    fn test_check_id() {
        let check = RequiredAfterOptionalCheck;
        assert_eq!(check.id(), "parameter.requiredAfterOptional");
    }
}
