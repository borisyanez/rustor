//! Generics type validation (Level 6)
//!
//! Validates that type arguments in generic types satisfy template bounds.
//! Reports generics.notSubtype errors like PHPStan.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
use crate::types::php_type::Type;
use crate::types::phpdoc::{parse_phpdoc, PhpDoc, TemplateParam};
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Check for invalid generic type arguments
pub struct GenericsCheck;

impl Check for GenericsCheck {
    fn id(&self) -> &'static str {
        "generics.notSubtype"
    }

    fn description(&self) -> &'static str {
        "Validates generic type arguments satisfy template bounds"
    }

    fn level(&self) -> u8 {
        6
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = GenericsVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            symbol_table: ctx.symbol_table,
            current_namespace: String::new(),
            use_fqn_map: HashMap::new(),
            // Map from class FQN to its template parameters
            class_templates: HashMap::new(),
            issues: Vec::new(),
        };

        // First pass: collect class definitions with @template annotations
        visitor.collect_templates(program);

        // Second pass: validate generic type usages in PHPDocs
        visitor.validate_generics(program);

        visitor.issues
    }
}

struct GenericsVisitor<'s> {
    source: &'s str,
    file_path: PathBuf,
    symbol_table: Option<&'s SymbolTable>,
    current_namespace: String,
    use_fqn_map: HashMap<String, String>,
    /// Class FQN -> list of template parameters (from local file)
    class_templates: HashMap<String, Vec<TemplateParam>>,
    issues: Vec<Issue>,
}

impl<'s> GenericsVisitor<'s> {
    fn collect_templates<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_templates_in_stmt(stmt, "");
        }
    }

    fn collect_templates_in_stmt<'a>(&mut self, stmt: &Statement<'a>, namespace: &str) {
        match stmt {
            Statement::Namespace(ns) => {
                let ns_name = if let Some(ref name) = ns.name {
                    let span = name.span();
                    self.source[span.start.offset as usize..span.end.offset as usize].to_string()
                } else {
                    String::new()
                };

                if self.current_namespace.is_empty() {
                    self.current_namespace = ns_name.clone();
                }

                match &ns.body {
                    NamespaceBody::Implicit(body) => {
                        for inner in body.statements.iter() {
                            self.collect_templates_in_stmt(inner, &ns_name);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.collect_templates_in_stmt(inner, &ns_name);
                        }
                    }
                }
            }
            Statement::Use(use_stmt) => {
                self.collect_use_imports(use_stmt);
            }
            Statement::Class(class) => {
                let name = &self.source
                    [class.name.span.start.offset as usize..class.name.span.end.offset as usize];
                let fqn = if namespace.is_empty() {
                    name.to_string()
                } else {
                    format!("{}\\{}", namespace, name)
                };

                // Check if this class has @template annotations
                if let Some(doc) = self.extract_phpdoc(class.span().start.offset as usize) {
                    if !doc.templates.is_empty() {
                        self.class_templates
                            .insert(fqn.to_lowercase(), doc.templates.clone());
                        // Also insert short name for local lookup
                        self.class_templates
                            .insert(name.to_lowercase(), doc.templates.clone());
                    }
                }
            }
            Statement::Interface(iface) => {
                let name = &self.source
                    [iface.name.span.start.offset as usize..iface.name.span.end.offset as usize];
                let fqn = if namespace.is_empty() {
                    name.to_string()
                } else {
                    format!("{}\\{}", namespace, name)
                };

                if let Some(doc) = self.extract_phpdoc(iface.span().start.offset as usize) {
                    if !doc.templates.is_empty() {
                        self.class_templates
                            .insert(fqn.to_lowercase(), doc.templates.clone());
                        self.class_templates
                            .insert(name.to_lowercase(), doc.templates.clone());
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_templates_in_stmt(inner, namespace);
                }
            }
            _ => {}
        }
    }

    fn collect_use_imports<'a>(&mut self, use_stmt: &Use<'a>) {
        let use_span = use_stmt.span();
        let use_text = &self.source[use_span.start.offset as usize..use_span.end.offset as usize];

        let text = use_text
            .trim_start_matches("use")
            .trim_start()
            .trim_start_matches("function")
            .trim_start_matches("const")
            .trim()
            .trim_end_matches(';')
            .trim();

        if let Some(brace_start) = text.find('{') {
            let prefix = text[..brace_start].trim().trim_end_matches('\\');
            if let Some(brace_end) = text.find('}') {
                let group_content = &text[brace_start + 1..brace_end];
                for item in group_content.split(',') {
                    let item = item.trim();
                    if let Some(as_pos) = item.to_lowercase().find(" as ") {
                        let class_part = item[..as_pos].trim();
                        let alias = item[as_pos + 4..].trim();
                        let fqn = format!("{}\\{}", prefix, class_part);
                        self.use_fqn_map.insert(alias.to_lowercase(), fqn);
                    } else {
                        let name = item.rsplit('\\').next().unwrap_or(item).trim();
                        if !name.is_empty() {
                            let fqn = format!("{}\\{}", prefix, item.trim());
                            self.use_fqn_map.insert(name.to_lowercase(), fqn);
                        }
                    }
                }
            }
        } else if let Some(as_pos) = text.to_lowercase().find(" as ") {
            let fqn = text[..as_pos].trim().to_string();
            let alias = text[as_pos + 4..].trim();
            self.use_fqn_map.insert(alias.to_lowercase(), fqn);
        } else {
            let name = text.rsplit('\\').next().unwrap_or(text).trim();
            if !name.is_empty() {
                self.use_fqn_map
                    .insert(name.to_lowercase(), text.to_string());
            }
        }
    }

    fn validate_generics<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.validate_stmt_generics(stmt);
        }
    }

    fn validate_stmt_generics<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                if let Some(doc) = self.extract_phpdoc(func.span().start.offset as usize) {
                    self.validate_phpdoc_generics(&doc, func.span().start.offset as usize, None);
                }
            }
            Statement::Class(class) => {
                let class_name = &self.source
                    [class.name.span.start.offset as usize..class.name.span.end.offset as usize];
                let class_fqn = if self.current_namespace.is_empty() {
                    class_name.to_string()
                } else {
                    format!("{}\\{}", self.current_namespace, class_name)
                };

                if let Some(doc) = self.extract_phpdoc(class.span().start.offset as usize) {
                    self.validate_phpdoc_generics(
                        &doc,
                        class.span().start.offset as usize,
                        Some(&class_fqn),
                    );
                }

                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Property(prop) => {
                            if let Some(doc) =
                                self.extract_phpdoc(prop.span().start.offset as usize)
                            {
                                self.validate_phpdoc_generics(
                                    &doc,
                                    prop.span().start.offset as usize,
                                    Some(&class_fqn),
                                );
                            }
                        }
                        ClassLikeMember::Method(method) => {
                            if let Some(doc) =
                                self.extract_phpdoc(method.span().start.offset as usize)
                            {
                                self.validate_phpdoc_generics(
                                    &doc,
                                    method.span().start.offset as usize,
                                    Some(&class_fqn),
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::Interface(iface) => {
                let iface_name = &self.source
                    [iface.name.span.start.offset as usize..iface.name.span.end.offset as usize];
                let iface_fqn = if self.current_namespace.is_empty() {
                    iface_name.to_string()
                } else {
                    format!("{}\\{}", self.current_namespace, iface_name)
                };

                if let Some(doc) = self.extract_phpdoc(iface.span().start.offset as usize) {
                    self.validate_phpdoc_generics(
                        &doc,
                        iface.span().start.offset as usize,
                        Some(&iface_fqn),
                    );
                }

                for member in iface.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let Some(doc) =
                            self.extract_phpdoc(method.span().start.offset as usize)
                        {
                            self.validate_phpdoc_generics(
                                &doc,
                                method.span().start.offset as usize,
                                Some(&iface_fqn),
                            );
                        }
                    }
                }
            }
            Statement::Namespace(ns) => {
                match &ns.body {
                    NamespaceBody::Implicit(body) => {
                        for inner in body.statements.iter() {
                            self.validate_stmt_generics(inner);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.validate_stmt_generics(inner);
                        }
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.validate_stmt_generics(inner);
                }
            }
            _ => {}
        }
    }

    fn validate_phpdoc_generics(
        &mut self,
        doc: &PhpDoc,
        offset: usize,
        _context_class: Option<&str>,
    ) {
        // Validate @param types
        for (param_name, param_type) in &doc.params {
            self.validate_type_generics(param_type, offset, Some(param_name.as_str()));
        }

        // Validate @return type
        if let Some(ref return_type) = doc.return_type {
            self.validate_type_generics(return_type, offset, None);
        }

        // Validate @var type
        if let Some(ref var_type) = doc.var_type {
            self.validate_type_generics(var_type, offset, None);
        }

        // Validate @property types
        for (_, prop_type, _) in &doc.properties {
            self.validate_type_generics(prop_type, offset, None);
        }
    }

    fn validate_type_generics(&mut self, ty: &Type, offset: usize, param_name: Option<&str>) {
        match ty {
            Type::GenericObject {
                class_name,
                type_args,
            } => {
                // Look up the class to find its template parameters
                let class_lower = class_name.to_lowercase();
                let resolved_fqn = self
                    .use_fqn_map
                    .get(&class_lower)
                    .cloned()
                    .unwrap_or_else(|| class_name.clone());

                // Try to find template params - first from local file, then symbol table
                let templates: Option<Vec<TemplateParam>> = self
                    .class_templates
                    .get(&resolved_fqn.to_lowercase())
                    .or_else(|| self.class_templates.get(&class_lower))
                    .cloned()
                    .or_else(|| {
                        // Look up in symbol table
                        self.symbol_table.and_then(|st| {
                            st.get_class(&resolved_fqn)
                                .or_else(|| st.get_class(class_name))
                                .filter(|ci| !ci.template_params.is_empty())
                                .map(|ci| ci.template_params.clone())
                        })
                    });

                if let Some(templates) = templates {
                    // Validate each type argument against its template bound
                    for (i, (template, type_arg)) in
                        templates.iter().zip(type_args.iter()).enumerate()
                    {
                        if let Some(ref bound) = template.bound {
                            if !self.type_satisfies_bound(type_arg, bound) {
                                let (line, col) = self.get_line_col(offset);

                                let type_arg_str = type_arg.to_string();
                                let generic_type_str = format!(
                                    "{}<{}>",
                                    class_name,
                                    type_args
                                        .iter()
                                        .map(|t| t.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                );

                                let message = if let Some(pname) = param_name {
                                    format!(
                                        "Type {} in generic type {} in PHPDoc tag @param for parameter ${} is not subtype of template type {} of {} of class {}.",
                                        type_arg_str,
                                        generic_type_str,
                                        pname,
                                        template.name,
                                        bound,
                                        resolved_fqn
                                    )
                                } else {
                                    format!(
                                        "Type {} in generic type {} is not subtype of template type {} of {} of class {}.",
                                        type_arg_str,
                                        generic_type_str,
                                        template.name,
                                        bound,
                                        resolved_fqn
                                    )
                                };

                                self.issues.push(
                                    Issue::error(
                                        "generics.notSubtype",
                                        message,
                                        self.file_path.clone(),
                                        line,
                                        col,
                                    )
                                    .with_identifier("generics.notSubtype"),
                                );
                            }
                        }
                    }
                }

                // Also validate nested generic types in type arguments
                for type_arg in type_args {
                    self.validate_type_generics(type_arg, offset, None);
                }
            }
            Type::Union(types) => {
                for t in types {
                    self.validate_type_generics(t, offset, param_name);
                }
            }
            Type::Intersection(types) => {
                for t in types {
                    self.validate_type_generics(t, offset, param_name);
                }
            }
            Type::Nullable(inner) => {
                self.validate_type_generics(inner, offset, param_name);
            }
            Type::Array { value, .. } => {
                self.validate_type_generics(value, offset, param_name);
            }
            Type::List { value } => {
                self.validate_type_generics(value, offset, param_name);
            }
            Type::Iterable { value, .. } => {
                self.validate_type_generics(value, offset, param_name);
            }
            _ => {}
        }
    }

    /// Check if a type argument satisfies a template bound
    fn type_satisfies_bound(&self, type_arg: &Type, bound: &Type) -> bool {
        // If bound is "object", any class type satisfies it
        if matches!(bound, Type::Object { class_name: None }) {
            return matches!(
                type_arg,
                Type::Object { class_name: Some(_) } | Type::GenericObject { .. }
            );
        }

        // If types are exactly equal, it satisfies
        if type_arg == bound {
            return true;
        }

        // Get class names
        let type_arg_class = type_arg.get_class_name();
        let bound_class = bound.get_class_name();

        match (type_arg_class, bound_class) {
            (Some(arg_name), Some(bound_name)) => {
                // Resolve arg_name using use map if it's not fully qualified
                let resolved_arg = if !arg_name.contains('\\') || arg_name.starts_with('\\') {
                    let clean_arg = arg_name.trim_start_matches('\\');
                    // Try to resolve via use map
                    self.use_fqn_map
                        .get(&clean_arg.to_lowercase())
                        .cloned()
                        .unwrap_or_else(|| {
                            // If not in use map and no namespace, assume global namespace
                            if !arg_name.contains('\\') {
                                // Could be a local class in same namespace, try to resolve
                                if !self.current_namespace.is_empty() {
                                    format!("{}\\{}", self.current_namespace, arg_name)
                                } else {
                                    arg_name.trim_start_matches('\\').to_string()
                                }
                            } else {
                                clean_arg.to_string()
                            }
                        })
                } else {
                    arg_name.to_string()
                };

                // Normalize both for comparison
                let arg_normalized = resolved_arg.to_lowercase().trim_start_matches('\\').to_string();
                let bound_normalized = bound_name.to_lowercase().trim_start_matches('\\').to_string();

                // Check exact FQN match
                if arg_normalized == bound_normalized {
                    return true;
                }

                // Check if the type arg class is a subtype of the bound class
                // (would need inheritance info from symbol table for full support)
                // For now, check if the short names match (partial match)
                let arg_short = arg_normalized.rsplit('\\').next().unwrap_or(&arg_normalized);
                let bound_short = bound_normalized.rsplit('\\').next().unwrap_or(&bound_normalized);

                // Only match if FQNs are the same
                // Different namespaces = different classes
                arg_normalized == bound_normalized
            }
            _ => {
                // For non-class types, check exact match
                type_arg == bound
            }
        }
    }

    fn extract_phpdoc(&self, offset: usize) -> Option<PhpDoc> {
        let before = &self.source[..offset];

        if let Some(doc_end) = before.rfind("*/") {
            if let Some(doc_start) = before[..doc_end].rfind("/**") {
                let between = &before[doc_end + 2..];
                let between_trimmed = between.trim();

                // Check that there's nothing significant between PHPDoc and element
                // Only allow: whitespace, #attributes, and visibility/modifier keywords
                let is_valid = between_trimmed.is_empty()
                    || self.is_only_modifiers_between(between_trimmed);

                if is_valid {
                    let doc_comment = &self.source[doc_start..doc_end + 2];
                    return Some(parse_phpdoc(doc_comment));
                }
            }
        }
        None
    }

    /// Check if the content between PHPDoc and element is only modifiers/attributes
    fn is_only_modifiers_between(&self, between: &str) -> bool {
        // If there are braces { or }, semicolons, or function/class keywords,
        // there are statements between the PHPDoc and current element
        if between.contains('{')
            || between.contains('}')
            || between.contains(';')
            || between.contains("function ")
            || between.contains("class ")
            || between.contains("interface ")
            || between.contains("trait ")
        {
            return false;
        }

        // Allow only modifiers and attributes
        let allowed_prefixes = [
            "public", "private", "protected", "static", "final", "abstract", "readonly", "#[",
        ];
        let trimmed = between.trim();
        if trimmed.is_empty() {
            return true;
        }

        // Check if it's just modifiers/attributes
        allowed_prefixes.iter().any(|p| trimmed.starts_with(p))
    }

    fn get_line_col(&self, offset: usize) -> (usize, usize) {
        let prefix = &self.source[..offset.min(self.source.len())];
        let line = prefix.matches('\n').count() + 1;
        let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = offset - last_newline + 1;
        (line, col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generics_check_level() {
        let check = GenericsCheck;
        assert_eq!(check.level(), 6);
        assert_eq!(check.id(), "generics.notSubtype");
    }
}
