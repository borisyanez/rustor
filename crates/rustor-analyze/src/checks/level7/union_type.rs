//! Union type checking (Level 7)
//!
//! When checkUnionTypes is enabled (level 7+), method calls and property access
//! on union types must be valid for ALL types in the union, not just SOME.
//!
//! Example that fails at level 7:
//! ```php
//! function foo(A|B $x) {
//!     $x->methodOnlyInA(); // ERROR: B doesn't have methodOnlyInA()
//! }
//! ```

use crate::checks::{Check, CheckContext};
use crate::issue::{Issue, Severity};
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::{HashMap, HashSet};

/// Check for invalid method/property access on union types
pub struct UnionTypeCheck;

impl Check for UnionTypeCheck {
    fn id(&self) -> &'static str {
        "unionType.invalid"
    }

    fn description(&self) -> &'static str {
        "Checks that methods/properties accessed on union types exist on all types in the union"
    }

    fn level(&self) -> u8 {
        7
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = UnionTypeVisitor {
            source: ctx.source,
            file_path: ctx.file_path,
            class_properties: HashMap::new(),
            class_methods: HashMap::new(),
            class_names: HashMap::new(),
            union_params: HashMap::new(),
            issues: Vec::new(),
        };

        // First pass: collect class properties and methods
        visitor.collect_definitions(program);

        // Second pass: analyze union type access
        visitor.analyze_program(program);

        visitor.issues
    }
}

struct UnionTypeVisitor<'s> {
    source: &'s str,
    file_path: &'s std::path::Path,
    /// Class name (lowercase) -> property names (lowercase)
    class_properties: HashMap<String, HashSet<String>>,
    /// Class name (lowercase) -> method names (lowercase)
    class_methods: HashMap<String, HashSet<String>>,
    /// Class name (lowercase) -> original case class name
    class_names: HashMap<String, String>,
    /// Parameter name -> list of types in union
    union_params: HashMap<String, Vec<String>>,
    issues: Vec<Issue>,
}

impl<'s> UnionTypeVisitor<'s> {
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

    /// Recursively extract all type names from a union hint
    fn extract_union_types<'a>(&self, hint: &Hint<'a>) -> Vec<String> {
        match hint {
            Hint::Union(union) => {
                let mut types = self.extract_union_types(&union.left);
                types.extend(self.extract_union_types(&union.right));
                types
            }
            Hint::Identifier(ident) => {
                vec![self.get_span_text(&ident.span()).to_string()]
            }
            Hint::Nullable(nullable) => {
                let mut types = self.extract_union_types(&nullable.hint);
                types.push("null".to_string());
                types
            }
            Hint::Null(_) => vec!["null".to_string()],
            Hint::String(_) => vec!["string".to_string()],
            Hint::Integer(_) => vec!["int".to_string()],
            Hint::Float(_) => vec!["float".to_string()],
            Hint::Bool(_) => vec!["bool".to_string()],
            Hint::Array(_) => vec!["array".to_string()],
            Hint::Object(_) => vec!["object".to_string()],
            Hint::Mixed(_) => vec!["mixed".to_string()],
            Hint::Callable(_) => vec!["callable".to_string()],
            Hint::Iterable(_) => vec!["iterable".to_string()],
            Hint::Void(_) => vec!["void".to_string()],
            Hint::Never(_) => vec!["never".to_string()],
            Hint::True(_) => vec!["true".to_string()],
            Hint::False(_) => vec!["false".to_string()],
            Hint::Parenthesized(paren) => self.extract_union_types(&paren.hint),
            Hint::Intersection(_) => Vec::new(), // Skip intersection types for now
            _ => Vec::new(),
        }
    }

    /// Check if a hint contains a union type
    fn is_union_hint(&self, hint: &Hint<'_>) -> bool {
        matches!(hint, Hint::Union(_)) || matches!(hint, Hint::Nullable(n) if matches!(&*n.hint, Hint::Union(_)))
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
                let mut properties = HashSet::new();
                let mut methods = HashSet::new();

                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Property(prop) => {
                            for var in prop.variables() {
                                let prop_name = self.get_span_text(&var.span);
                                let name = prop_name.trim_start_matches('$').to_lowercase();
                                properties.insert(name);
                            }
                        }
                        ClassLikeMember::Method(method) => {
                            let method_name = self.get_span_text(&method.name.span).to_lowercase();
                            methods.insert(method_name.clone());

                            // Also collect promoted properties from constructor
                            if method_name == "__construct" {
                                for param in method.parameter_list.parameters.iter() {
                                    if param.is_promoted_property() {
                                        let prop_name = self.get_span_text(&param.variable.span);
                                        let name = prop_name.trim_start_matches('$').to_lowercase();
                                        properties.insert(name);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                self.class_names.insert(class_lower.clone(), original_name);
                self.class_properties.insert(class_lower.clone(), properties);
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
                let old_union_params = self.union_params.clone();

                // Collect union type parameters
                self.union_params.clear();
                for param in func.parameter_list.parameters.iter() {
                    if let Some(hint) = &param.hint {
                        if self.is_union_hint(hint) {
                            let param_name = self.get_span_text(&param.variable.span()).trim_start_matches('$');
                            let types = self.extract_union_types(hint);
                            if types.len() > 1 {
                                self.union_params.insert(param_name.to_string(), types);
                            }
                        }
                    }
                }

                // Visit function body
                for inner in func.body.statements.iter() {
                    self.visit_body_statement(inner);
                }

                // Restore old state
                self.union_params = old_union_params;
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        match &method.body {
                            MethodBody::Concrete(concrete) => {
                                // Save old state
                                let old_union_params = self.union_params.clone();

                                // Collect union type parameters
                                self.union_params.clear();
                                for param in method.parameter_list.parameters.iter() {
                                    if let Some(hint) = &param.hint {
                                        if self.is_union_hint(hint) {
                                            let param_name = self.get_span_text(&param.variable.span()).trim_start_matches('$');
                                            let types = self.extract_union_types(hint);
                                            if types.len() > 1 {
                                                self.union_params.insert(param_name.to_string(), types);
                                            }
                                        }
                                    }
                                }

                                // Visit method body
                                for inner in concrete.statements.iter() {
                                    self.visit_body_statement(inner);
                                }

                                // Restore old state
                                self.union_params = old_union_params;
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
            Expression::Access(access) => {
                match access {
                    Access::Property(prop) => {
                        self.check_union_property_access(&prop.object, &prop.property.span());
                        self.visit_expression(&prop.object);
                    }
                    Access::NullSafeProperty(prop) => {
                        self.visit_expression(&prop.object);
                    }
                    _ => {}
                }
            }
            Expression::Call(call) => {
                match call {
                    Call::Method(method) => {
                        self.check_union_method_access(&method.object, &method.method.span());
                        self.visit_expression(&method.object);
                    }
                    Call::NullSafeMethod(_) => {}
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

    fn check_union_property_access<'a>(
        &mut self,
        target: &Expression<'a>,
        property_span: &mago_span::Span,
    ) {
        if let Expression::Variable(var) = target {
            let var_name = self.get_span_text(&var.span()).trim_start_matches('$');

            if let Some(types) = self.union_params.get(var_name).cloned() {
                let property_name = self.get_span_text(property_span).to_lowercase();
                self.check_member_on_all_types(&types, &property_name, "property", target.span().start.offset as usize);
            }
        }
    }

    fn check_union_method_access<'a>(
        &mut self,
        target: &Expression<'a>,
        method_span: &mago_span::Span,
    ) {
        if let Expression::Variable(var) = target {
            let var_name = self.get_span_text(&var.span()).trim_start_matches('$');

            if let Some(types) = self.union_params.get(var_name).cloned() {
                let method_name = self.get_span_text(method_span).to_lowercase();
                self.check_member_on_all_types(&types, &method_name, "method", target.span().start.offset as usize);
            }
        }
    }

    fn check_member_on_all_types(
        &mut self,
        types: &[String],
        member_name: &str,
        member_type: &str,
        offset: usize,
    ) {
        let mut missing_on = Vec::new();

        for type_name in types {
            let type_lower = type_name.to_lowercase();

            // Skip built-in scalar types and special types
            if matches!(type_lower.as_str(), "null" | "string" | "int" | "float" | "bool" | "array" | "mixed" | "callable" | "iterable" | "void" | "never" | "true" | "false" | "object") {
                // Scalar types and null don't have properties/methods
                if type_lower == "null" || type_lower == "int" || type_lower == "string" || type_lower == "float" || type_lower == "bool" || type_lower == "array" {
                    missing_on.push(type_name.clone());
                }
                continue;
            }

            // Check if the member exists on this type
            let has_member = if member_type == "property" {
                self.class_properties.get(&type_lower)
                    .map(|props| props.contains(member_name))
                    .unwrap_or(false)
            } else {
                self.class_methods.get(&type_lower)
                    .map(|methods| methods.contains(member_name))
                    .unwrap_or(false)
            };

            if !has_member {
                missing_on.push(type_name.clone());
            }
        }

        // Report error if member is missing on any type
        if !missing_on.is_empty() {
            let (line, col) = self.get_line_col(offset);
            let union_str = types.join("|");
            let missing_str = missing_on.join(", ");

            let message = if missing_on.len() == types.len() {
                format!(
                    "Cannot access {} {} on {}, {} doesn't exist on any type in the union",
                    member_type, member_name, union_str, member_name
                )
            } else {
                format!(
                    "Cannot access {} {} on {}, missing on: {}",
                    member_type, member_name, union_str, missing_str
                )
            };

            self.issues.push(Issue {
                check_id: "unionType.invalid".to_string(),
                severity: Severity::Error,
                message,
                file: self.file_path.to_path_buf(),
                line,
                column: col,
                identifier: Some(if member_type == "property" {
                    "property.notFoundInUnion".to_string()
                } else {
                    "method.notFoundInUnion".to_string()
                }),
                tip: Some(format!(
                    "Ensure {} {} exists on all types in the union: {}",
                    member_type, member_name, union_str
                )),
            });
        }
    }
}
