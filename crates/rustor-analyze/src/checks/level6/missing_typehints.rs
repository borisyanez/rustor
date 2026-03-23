//! Missing typehints detection (Level 6)
//!
//! Detects:
//! - Properties without type declarations
//! - Function/method parameters without type hints
//! - Functions/methods without return type hints
//!
//! This check respects PHPDoc annotations - if a type is specified via @param or @return,
//! it won't report a missing typehint error.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::types::phpdoc::{parse_phpdoc, PhpDoc};
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
            symbol_table: ctx.symbol_table,
            issues: Vec::new(),
            current_namespace: String::new(),
        };

        visitor.analyze_program(program);
        visitor.issues
    }
}

struct MissingTypehintVisitor<'s> {
    source: &'s str,
    file_path: PathBuf,
    #[allow(dead_code)]
    symbol_table: Option<&'s crate::symbols::SymbolTable>,
    issues: Vec<Issue>,
    current_namespace: String,
}

impl<'s> MissingTypehintVisitor<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Extract PHPDoc comment that precedes the given offset
    fn extract_phpdoc(&self, offset: usize) -> Option<PhpDoc> {
        // Look backwards from offset to find a doc comment
        let before = &self.source[..offset];

        // Find the last occurrence of /** (start of PHPDoc)
        if let Some(doc_end) = before.rfind("*/") {
            if let Some(doc_start) = before[..doc_end].rfind("/**") {
                // Make sure there's no code between the doc and the function
                let between = &before[doc_end + 2..];
                let between_trimmed = between.trim();

                // Only allow whitespace, attributes (#[...]), and modifiers between doc and function
                // Must not contain braces (which indicate other code between doc and function)
                let has_braces = between.contains('{') || between.contains('}');
                let is_valid = !has_braces && (
                    between_trimmed.is_empty()
                    || between_trimmed.chars().all(|c| c.is_whitespace())
                    || between_trimmed.starts_with('#')  // PHP attributes
                    || between_trimmed.starts_with("public")
                    || between_trimmed.starts_with("private")
                    || between_trimmed.starts_with("protected")
                    || between_trimmed.starts_with("static")
                    || between_trimmed.starts_with("final")
                    || between_trimmed.starts_with("abstract")
                    || between_trimmed.starts_with("readonly")
                );

                if is_valid {
                    let doc_comment = &self.source[doc_start..doc_end + 2];
                    return Some(parse_phpdoc(doc_comment));
                }
            }
        }
        None
    }

    /// Check if a parameter has its type specified in PHPDoc
    fn has_phpdoc_param_type(&self, phpdoc: &PhpDoc, param_name: &str) -> bool {
        // Remove $ prefix for comparison
        let name = param_name.trim_start_matches('$');
        phpdoc.params.iter().any(|(n, ty)| {
            n == name && !matches!(ty, crate::types::php_type::Type::Mixed)
        })
    }

    /// Check if a parameter has a SPECIFIC array value type in PHPDoc (for iterableValue check)
    /// Returns true only if the PHPDoc type specifies array value types (e.g., string[], array<int>)
    /// Returns false for plain "array" (which has no specific value type)
    fn has_phpdoc_iterable_value_type(&self, phpdoc: &PhpDoc, param_name: &str) -> bool {
        let name = param_name.trim_start_matches('$');
        phpdoc.params.iter().any(|(n, ty)| {
            if n != name {
                return false;
            }
            // Must have SPECIFIC value type, not just plain array
            Self::type_has_iterable_value(ty) && !Self::is_plain_array_type(ty)
        })
    }

    /// Check if a PHPDoc Type is a plain array without specific value type
    fn is_plain_array_type(ty: &crate::types::php_type::Type) -> bool {
        use crate::types::php_type::Type;
        match ty {
            Type::Array { key, value } => {
                matches!(**key, Type::Mixed) && matches!(**value, Type::Mixed)
            }
            Type::Union(types) => types.iter().any(|t| Self::is_plain_array_type(t)),
            Type::Nullable(inner) => Self::is_plain_array_type(inner),
            _ => false,
        }
    }

    /// Recursively check if a type contains an iterable with value type specified
    fn type_has_iterable_value(ty: &crate::types::php_type::Type) -> bool {
        use crate::types::php_type::Type;
        match ty {
            Type::Array { .. } | Type::List { .. } | Type::Iterable { .. }
            | Type::NonEmptyArray { .. } => true,
            Type::Nullable(inner) => Self::type_has_iterable_value(inner),
            Type::Union(types) => types.iter().any(|t| Self::type_has_iterable_value(t)),
            Type::Intersection(types) => types.iter().any(|t| Self::type_has_iterable_value(t)),
            // string[] parses to Type::Array, but also check Object with known collection class names
            Type::Object { class_name: Some(name) } => {
                let lower = name.to_lowercase();
                lower.contains("collection") || lower.contains("iterator")
                    || lower.contains("traversable") || lower.contains("generator")
            }
            _ => false,
        }
    }

    /// Check if return type is specified in PHPDoc
    fn has_phpdoc_return_type(&self, phpdoc: &PhpDoc) -> bool {
        phpdoc.return_type.is_some()
    }

    /// Check if return type has iterable value type in PHPDoc
    fn has_phpdoc_return_iterable_value_type(&self, phpdoc: &PhpDoc) -> bool {
        phpdoc.return_type.as_ref().map_or(false, |ty| {
            Self::type_has_iterable_value(ty) && !Self::is_plain_array_type(ty)
        })
    }

    /// Check if @var has iterable value type in PHPDoc (for properties)
    fn has_phpdoc_var_iterable_value_type(&self, phpdoc: &PhpDoc) -> bool {
        phpdoc.var_type.as_ref().map_or(false, |ty| {
            Self::type_has_iterable_value(ty) && !Self::is_plain_array_type(ty)
        })
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
                let short_name = self.get_span_text(&func.name.span).to_string();
                let func_name = if self.current_namespace.is_empty() {
                    short_name
                } else {
                    format!("{}\\{}", self.current_namespace, short_name)
                };
                self.check_function(&func_name, &func.parameter_list, &func.return_type_hint, func.span());
            }
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();
                let fqn = if self.current_namespace.is_empty() {
                    class_name.clone()
                } else {
                    format!("{}\\{}", self.current_namespace, class_name)
                };

                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Property(prop) => {
                            self.check_property(&fqn, prop);
                        }
                        ClassLikeMember::Method(method) => {
                            let method_name = self.get_span_text(&method.name.span).to_string();
                            let full_name = format!("{}::{}", fqn, method_name);

                            // Skip magic methods except __construct (which should check parameter types)
                            if method_name.starts_with("__") && method_name != "__construct" {
                                continue;
                            }

                            self.check_method(&full_name, &method.parameter_list, &method.return_type_hint, method.span());
                        }
                        _ => {}
                    }
                }
            }
            Statement::Interface(iface) => {
                let iface_name = self.get_span_text(&iface.name.span).to_string();
                let fqn = if self.current_namespace.is_empty() {
                    iface_name.clone()
                } else {
                    format!("{}\\{}", self.current_namespace, iface_name)
                };

                for member in iface.members.iter() {
                    match member {
                        ClassLikeMember::Method(method) => {
                            let method_name = self.get_span_text(&method.name.span).to_string();
                            let full_name = format!("{}::{}", fqn, method_name);

                            if method_name.starts_with("__") && method_name != "__construct" {
                                continue;
                            }

                            self.check_method(&full_name, &method.parameter_list, &method.return_type_hint, method.span());
                        }
                        _ => {}
                    }
                }
            }
            Statement::Trait(trt) => {
                let trait_name = self.get_span_text(&trt.name.span).to_string();
                let fqn = if self.current_namespace.is_empty() {
                    trait_name.clone()
                } else {
                    format!("{}\\{}", self.current_namespace, trait_name)
                };

                for member in trt.members.iter() {
                    match member {
                        ClassLikeMember::Property(prop) => {
                            self.check_property(&fqn, prop);
                        }
                        ClassLikeMember::Method(method) => {
                            let method_name = self.get_span_text(&method.name.span).to_string();
                            let full_name = format!("{}::{}", fqn, method_name);

                            if method_name.starts_with("__") && method_name != "__construct" {
                                continue;
                            }

                            self.check_method(&full_name, &method.parameter_list, &method.return_type_hint, method.span());
                        }
                        _ => {}
                    }
                }
            }
            Statement::Namespace(ns) => {
                let prev_namespace = self.current_namespace.clone();
                if let Some(ref name) = ns.name {
                    let span = name.span();
                    self.current_namespace = self.get_span_text(&span).to_string();
                }
                match &ns.body {
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
                }
                self.current_namespace = prev_namespace;
            }
            Statement::Enum(enum_def) => {
                let enum_name = self.get_span_text(&enum_def.name.span).to_string();
                let fqn = if self.current_namespace.is_empty() {
                    enum_name.clone()
                } else {
                    format!("{}\\{}", self.current_namespace, enum_name)
                };
                for member in enum_def.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span).to_string();
                        let full_name = format!("{}::{}", fqn, method_name);
                        if method_name.starts_with("__") && method_name != "__construct" {
                            continue;
                        }
                        self.check_method(&full_name, &method.parameter_list, &method.return_type_hint, method.span());
                    }
                }
            }
            // Traverse control flow to find nested functions
            Statement::If(if_stmt) => {
                match &if_stmt.body {
                    IfBody::Statement(body) => {
                        self.analyze_statement(body.statement);
                        for clause in body.else_if_clauses.iter() {
                            self.analyze_statement(clause.statement);
                        }
                        if let Some(else_clause) = &body.else_clause {
                            self.analyze_statement(else_clause.statement);
                        }
                    }
                    IfBody::ColonDelimited(body) => {
                        for inner in body.statements.iter() { self.analyze_statement(inner); }
                    }
                }
            }
            Statement::While(w) => {
                match &w.body {
                    WhileBody::Statement(s) => self.analyze_statement(s),
                    WhileBody::ColonDelimited(b) => {
                        for inner in b.statements.iter() { self.analyze_statement(inner); }
                    }
                }
            }
            Statement::For(f) => {
                match &f.body {
                    ForBody::Statement(s) => self.analyze_statement(s),
                    ForBody::ColonDelimited(b) => {
                        for inner in b.statements.iter() { self.analyze_statement(inner); }
                    }
                }
            }
            Statement::Foreach(f) => {
                match &f.body {
                    ForeachBody::Statement(s) => self.analyze_statement(s),
                    ForeachBody::ColonDelimited(b) => {
                        for inner in b.statements.iter() { self.analyze_statement(inner); }
                    }
                }
            }
            Statement::Try(t) => {
                for inner in t.block.statements.iter() { self.analyze_statement(inner); }
                for catch in t.catch_clauses.iter() {
                    for inner in catch.block.statements.iter() { self.analyze_statement(inner); }
                }
                if let Some(finally) = &t.finally_clause {
                    for inner in finally.block.statements.iter() { self.analyze_statement(inner); }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() { self.analyze_statement(inner); }
            }
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
        // Extract PHPDoc for this function
        let phpdoc = self.extract_phpdoc(span.start.offset as usize);

        // Check for @param references to non-existent parameters (parameter.notFound)
        if let Some(ref doc) = phpdoc {
            let actual_params: Vec<String> = params.parameters.iter()
                .map(|p| self.get_span_text(&p.variable.span).trim_start_matches('$').to_string())
                .collect();
            for (doc_param_name, _) in &doc.params {
                if !actual_params.iter().any(|p| p == doc_param_name) {
                    let (line, col) = self.get_line_col(span.start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "parameter.notFound",
                            format!(
                                "PHPDoc tag @param references unknown parameter: ${}",
                                doc_param_name
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("parameter.notFound"),
                    );
                }
            }
        }

        // Check parameters
        for param in params.parameters.iter() {
            let param_name = self.get_span_text(&param.variable.span).to_string();

            if param.hint.is_none() {
                // Skip if PHPDoc has type for this param
                if let Some(ref doc) = phpdoc {
                    if self.has_phpdoc_param_type(doc, &param_name) {
                        // PHPDoc has @param — check if it's a plain array (iterableValue)
                        let clean_name = param_name.trim_start_matches('$');
                        for (pname, ptype) in &doc.params {
                            if pname == clean_name && Self::is_plain_array_type(ptype) {
                                let (line, col) = self.get_line_col(span.start.offset as usize);
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
                        continue;
                    }
                }

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
                    // Skip if PHPDoc has iterable value type for this param
                    if let Some(ref doc) = phpdoc {
                        if self.has_phpdoc_iterable_value_type(doc, &param_name) {
                            continue;
                        }
                    }

                    let (line, col) = self.get_line_col(span.start.offset as usize);
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
                    let (line, col) = self.get_line_col(span.start.offset as usize);
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
            // Skip if PHPDoc has return type
            if let Some(ref doc) = phpdoc {
                if self.has_phpdoc_return_type(doc) {
                    // PHPDoc has @return — check if it's a plain array (iterableValue)
                    if let Some(ref rt) = doc.return_type {
                        if Self::is_plain_array_type(rt) {
                            let (line, col) = self.get_line_col(span.start.offset as usize);
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
                    }
                    return;
                }
            }

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
                // Skip if PHPDoc has iterable value type for return
                if let Some(ref doc) = phpdoc {
                    if self.has_phpdoc_return_iterable_value_type(doc) {
                        return;
                    }
                }
                let (line, col) = self.get_line_col(span.start.offset as usize);
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
                let (line, col) = self.get_line_col(span.start.offset as usize);
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

    /// Check if a property has ORM/Doctrine attributes (Column, Id, JoinColumn, etc.)
    fn has_orm_attribute(&self, offset: usize) -> bool {
        let search_start = if offset > 300 { offset - 300 } else { 0 };
        let before = &self.source[search_start..offset];
        // Look for PHP 8 attributes related to Doctrine/ORM
        // Pattern: #[Column(...)] or #[ORM\Column(...)] or #[Id] etc.
        if let Some(attr_end) = before.rfind(']') {
            if let Some(attr_start) = before[..attr_end].rfind("#[") {
                let attr_text = &before[attr_start..attr_end + 1];
                // Check for Doctrine ORM attributes
                if attr_text.contains("Column") || attr_text.contains("Id")
                    || attr_text.contains("JoinColumn") || attr_text.contains("ManyToOne")
                    || attr_text.contains("OneToMany") || attr_text.contains("ManyToMany")
                    || attr_text.contains("OneToOne") || attr_text.contains("GeneratedValue")
                    || attr_text.contains("Embedded") || attr_text.contains("Version")
                {
                    return true;
                }
            }
        }
        false
    }

    /// Check for inline @var annotation before a parameter (PHP 8.0 promoted properties)
    /// Pattern: /** @var array<Type> */ private array $param
    fn has_inline_var_annotation(&self, offset: usize) -> bool {
        // Look backwards from offset for a close */ and then /**
        let before = &self.source[..offset];
        // Find the last */ within 200 chars before the parameter
        let search_start = if offset > 200 { offset - 200 } else { 0 };
        let search_area = &self.source[search_start..offset];
        if let Some(doc_end_pos) = search_area.rfind("*/") {
            if let Some(doc_start_pos) = search_area[..doc_end_pos].rfind("/**") {
                let doc_content = &search_area[doc_start_pos..doc_end_pos + 2];
                // Check if it contains @var with typed array
                if doc_content.contains("@var") {
                    // Check for typed arrays: array<, Type[], list<, etc.
                    if doc_content.contains("array<") || doc_content.contains("[]")
                        || doc_content.contains("list<") || doc_content.contains("Collection<")
                        || doc_content.contains("iterable<")
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a method overrides a parent/interface method that also has untyped params
    /// PHPStan only reports on the declaration, not the implementation
    /// Only skip if the parent/interface is also being analyzed (methods_fully_collected)
    fn method_overrides_parent_untyped(&self, class_fqn: &str, method_name: &str) -> bool {
        if let Some(symbol_table) = self.symbol_table {
            if let Some(class_info) = symbol_table.get_class(class_fqn) {
                // Check interfaces — only if the interface has fully collected methods
                // (meaning we'll also report on the interface itself)
                for iface in &class_info.interfaces {
                    if let Some(iface_info) = symbol_table.get_class(iface) {
                        if iface_info.methods_fully_collected {
                            if let Some(_method) = iface_info.get_method(method_name) {
                                return true;
                            }
                        }
                    }
                }
                // Check parent — only if parent has fully collected methods
                if let Some(parent) = &class_info.parent {
                    if let Some(parent_info) = symbol_table.get_class(parent) {
                        if parent_info.methods_fully_collected {
                            if let Some(_method) = parent_info.get_method(method_name) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn check_method<'a>(
        &mut self,
        method_name: &str,
        params: &FunctionLikeParameterList<'a>,
        return_type: &Option<FunctionLikeReturnTypeHint<'a>>,
        span: mago_span::Span,
    ) {
        // Extract PHPDoc for this method
        let phpdoc = self.extract_phpdoc(span.start.offset as usize);

        // Check for @param references to non-existent parameters (parameter.notFound)
        if let Some(ref doc) = phpdoc {
            let actual_params: Vec<String> = params.parameters.iter()
                .map(|p| self.get_span_text(&p.variable.span).trim_start_matches('$').to_string())
                .collect();
            for (doc_param_name, _) in &doc.params {
                if !actual_params.iter().any(|p| p == doc_param_name) {
                    let (line, col) = self.get_line_col(span.start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "parameter.notFound",
                            format!(
                                "PHPDoc tag @param references unknown parameter: ${}",
                                doc_param_name
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("parameter.notFound"),
                    );
                }
            }
        }

        // Check parameters
        for param in params.parameters.iter() {
            let param_name = self.get_span_text(&param.variable.span).to_string();

            if param.hint.is_none() {
                // Skip if PHPDoc has type for this param
                if let Some(ref doc) = phpdoc {
                    if self.has_phpdoc_param_type(doc, &param_name) {
                        // PHPDoc has @param type — check if it's a plain array (iterableValue)
                        let clean_name = param_name.trim_start_matches('$');
                        for (pname, ptype) in &doc.params {
                            if pname == clean_name && Self::is_plain_array_type(ptype) {
                                let (line, col) = self.get_line_col(span.start.offset as usize);
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
                        continue;
                    }
                }

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
                    // Skip if PHPDoc has iterable value type for this param
                    if let Some(ref doc) = phpdoc {
                        if self.has_phpdoc_iterable_value_type(doc, &param_name) {
                            continue;
                        }
                    }

                    // Also check for inline @var annotation before the parameter
                    // Pattern: /** @var array<Type> */ private array $param
                    if self.has_inline_var_annotation(param.span().start.offset as usize) {
                        continue;
                    }

                    let (line, col) = self.get_line_col(span.start.offset as usize);
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
                    let (line, col) = self.get_line_col(span.start.offset as usize);
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

        // Check return type (skip for constructors - they implicitly return void)
        let is_constructor = method_name.ends_with("::__construct");
        if return_type.is_none() && !is_constructor {
            // Skip if PHPDoc has return type
            if let Some(ref doc) = phpdoc {
                if self.has_phpdoc_return_type(doc) {
                    // PHPDoc has @return — but check if it's a plain array (iterableValue)
                    if let Some(ref rt) = doc.return_type {
                        if Self::is_plain_array_type(rt) {
                            let (line, col) = self.get_line_col(span.start.offset as usize);
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
                    }
                    return;
                }
            }

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
                // Skip if PHPDoc has iterable value type for return
                if let Some(ref doc) = phpdoc {
                    if self.has_phpdoc_return_iterable_value_type(doc) {
                        return;
                    }
                }

                let (line, col) = self.get_line_col(span.start.offset as usize);
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
                let (line, col) = self.get_line_col(span.start.offset as usize);
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
            // PHPStan does NOT report iterableValue for nullable arrays (?array)
            Hint::Nullable(_) => false,
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

        // Extract the type name from the hint span and resolve to FQN
        let raw_name = self.get_span_text(&hint.span()).to_string();
        let type_name = if raw_name.contains('\\') {
            // Already qualified — strip leading backslash
            raw_name.trim_start_matches('\\').to_string()
        } else if let Some(symbol_table) = self.symbol_table {
            // Try to resolve via symbol table using file alias map
            let ns = if self.current_namespace.is_empty() { None } else { Some(self.current_namespace.as_str()) };
            let resolved = symbol_table.resolve_class_name(
                &raw_name,
                &self.file_path,
                ns,
            );
            if resolved != raw_name.to_lowercase() {
                resolved
            } else if !self.current_namespace.is_empty() {
                // Try namespace prefix
                let fqn = format!("{}\\{}", self.current_namespace, raw_name);
                if symbol_table.get_class(&fqn).is_some() {
                    fqn
                } else {
                    raw_name
                }
            } else {
                raw_name
            }
        } else {
            raw_name
        };

        // Normalize to check against known generic classes (case-insensitive, check suffix)
        let type_lower = type_name.to_lowercase();

        // Common generic classes and their template parameters
        // Format: (class name/suffix to match, template parameter description)
        // Matches both short names (via use) and fully qualified names
        let generic_classes = [
            ("arrayiterator", "TKey, TValue"),
            ("iterator", "TKey, TValue"),
            ("traversable", "TKey, TValue"),
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
        // Extract PHPDoc for this property
        let phpdoc = self.extract_phpdoc(prop.span().start.offset as usize);

        // Check if property has a type
        if prop.hint().is_none() {
            // Skip if PHPDoc has @var type
            if let Some(ref doc) = phpdoc {
                if doc.var_type.is_some() {
                    return;
                }
            }

            for var in prop.variables() {
                let prop_name = self.get_span_text(&var.span);
                // Use variable span for line number (not property span which includes attributes)
                let (line, col) = self.get_line_col(var.span.start.offset as usize);
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
                // Skip if PHPDoc @var has iterable value type
                if let Some(ref doc) = phpdoc {
                    if self.has_phpdoc_var_iterable_value_type(doc) {
                        return;
                    }
                }

                for var in prop.variables() {
                    let prop_name = self.get_span_text(&var.span);
                    let (line, col) = self.get_line_col(var.span.start.offset as usize);
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
                    let (line, col) = self.get_line_col(var.span.start.offset as usize);
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
