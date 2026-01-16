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

                // Check for generic class without type parameters (missingType.generics)
                if let Some((class_name, template_params)) = self.is_generic_without_params(hint) {
                    let param_name = self.get_span_text(&param.variable.span);
                    let (line, col) = self.get_line_col(hint.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "missingType.generics",
                            format!(
                                "Function {}() has parameter {} with generic class {} but does not specify its types: {}",
                                func_name, param_name, class_name, template_params
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("missingType.generics"),
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
        } else if let Some(ret_type) = return_type {
            // Check if return type is a plain iterable without value type
            if self.is_plain_iterable_type(&ret_type.hint) {
                let (line, col) = self.get_line_col(ret_type.hint.span().start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "missingType.iterableValue",
                        format!(
                            "Function {}() return type has no value type specified in iterable type array.",
                            func_name
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("missingType.iterableValue"),
                );
            }

            // Check if return type is a generic class without type parameters
            if let Some((class_name, template_params)) = self.is_generic_without_params(&ret_type.hint) {
                let (line, col) = self.get_line_col(ret_type.hint.span().start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "missingType.generics",
                        format!(
                            "Function {}() return type with generic class {} does not specify its types: {}",
                            func_name, class_name, template_params
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("missingType.generics"),
                );
            }
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

                // Check for generic class without type parameters (missingType.generics)
                if let Some((class_name, template_params)) = self.is_generic_without_params(hint) {
                    let param_name = self.get_span_text(&param.variable.span);
                    let (line, col) = self.get_line_col(hint.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "missingType.generics",
                            format!(
                                "Method {}() has parameter {} with generic class {} but does not specify its types: {}",
                                method_name, param_name, class_name, template_params
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("missingType.generics"),
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
        } else if let Some(ret_type) = return_type {
            // Check if return type is a plain iterable without value type
            if self.is_plain_iterable_type(&ret_type.hint) {
                let (line, col) = self.get_line_col(ret_type.hint.span().start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "missingType.iterableValue",
                        format!(
                            "Method {}() return type has no value type specified in iterable type array.",
                            method_name
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("missingType.iterableValue"),
                );
            }

            // Check if return type is a generic class without type parameters
            if let Some((class_name, template_params)) = self.is_generic_without_params(&ret_type.hint) {
                let (line, col) = self.get_line_col(ret_type.hint.span().start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "missingType.generics",
                        format!(
                            "Method {}() return type with generic class {} does not specify its types: {}",
                            method_name, class_name, template_params
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("missingType.generics"),
                );
            }
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

    /// Check if a type hint is a generic class without type parameters specified
    /// Returns Some((class_name, template_params)) if it's a generic class missing type args
    fn is_generic_without_params(&self, hint: &Hint<'_>) -> Option<(String, String)> {
        // For nullable or parenthesized hints, check the inner type
        match hint {
            Hint::Nullable(nullable) => {
                return self.is_generic_without_params(&nullable.hint);
            }
            Hint::Parenthesized(p) => {
                return self.is_generic_without_params(&p.hint);
            }
            _ => {}
        };

        // Extract the type name from the hint span
        let type_name = self.get_span_text(&hint.span()).to_string();

        // Normalize to check against known generic classes (case-insensitive, check suffix)
        let type_lower = type_name.to_lowercase();

        // Common generic classes and their template parameters
        // Format: (class name/suffix to match, template parameter description)
        // Matches both short names (via use) and fully qualified names
        let generic_classes = [
            ("arrayiterator", "TKey, TValue"),
            ("iterator", "TKey, TValue"),
            ("traversable", "TKey, TValue"),
            ("generator", "TKey, TValue, TSend, TReturn"),
            ("app", "TContainerInterface"),  // Slim\App
            ("entityrepository", "T"),  // Doctrine\ORM\EntityRepository
            ("persistentcollection", "TKey, T"),  // Doctrine\ORM\PersistentCollection
            ("collection", "TKey, T"),  // Doctrine\Common\Collections\Collection
            ("arraycollection", "TKey, T"),  // Doctrine\Common\Collections\ArrayCollection
            ("abstractlazycollection", "TKey, T"),  // Doctrine\Common\Collections\AbstractLazyCollection
            ("objectrepository", "T"),  // Doctrine\Persistence\ObjectRepository
        ];

        for (class_pattern, template_params) in &generic_classes {
            // Match if the type name ends with the pattern (for namespaced classes)
            // or equals the pattern (for global classes)
            if type_lower.ends_with(class_pattern) || type_lower == *class_pattern {
                return Some((type_name, template_params.to_string()));
            }
        }

        None
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
        } else if let Some(hint) = prop.hint() {
            // Check if property type is a plain iterable without value type
            if self.is_plain_iterable_type(hint) {
                for var in prop.variables() {
                    let prop_name = self.get_span_text(&var.span);
                    let (line, col) = self.get_line_col(hint.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "missingType.iterableValue",
                            format!(
                                "Property {}::{} has no value type specified in iterable type array.",
                                class_name, prop_name
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("missingType.iterableValue"),
                    );
                }
            }

            // Check if property type is a generic class without type parameters
            if let Some((gen_class_name, template_params)) = self.is_generic_without_params(hint) {
                for var in prop.variables() {
                    let prop_name = self.get_span_text(&var.span);
                    let (line, col) = self.get_line_col(hint.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "missingType.generics",
                            format!(
                                "Property {}::{} with generic class {} does not specify its types: {}",
                                class_name, prop_name, gen_class_name, template_params
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("missingType.generics"),
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
    fn test_missing_typehint_check_level() {
        let check = MissingTypehintCheck;
        assert_eq!(check.level(), 6);
    }
}
