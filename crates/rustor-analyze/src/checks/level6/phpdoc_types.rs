//! PHPDoc type validation (Level 6)
//!
//! Validates that class names referenced in PHPDoc annotations (@param, @return, @var, @throws)
//! actually exist. This check is similar to PHPStan's class.notFound check for PHPDoc types.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
use crate::types::php_type::Type;
use crate::types::phpdoc::{parse_phpdoc, PhpDoc};
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Check for invalid class references in PHPDoc annotations
pub struct PhpDocTypesCheck;

impl Check for PhpDocTypesCheck {
    fn id(&self) -> &'static str {
        "class.notFound"
    }

    fn description(&self) -> &'static str {
        "Validates class references in PHPDoc annotations"
    }

    fn level(&self) -> u8 {
        0  // PHPStan reports PHPDoc class.notFound at level 0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = PhpDocTypesVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            builtin_classes: ctx.builtin_classes,
            symbol_table: ctx.symbol_table,
            current_namespace: String::new(),
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
            use_fqn_map: HashMap::new(),
            template_params: HashSet::new(),
            issues: Vec::new(),
        };

        // First pass: collect namespace, class definitions, and imports
        visitor.collect_definitions(program);

        // Second pass: validate PHPDoc types
        visitor.validate_phpdocs(program);

        visitor.issues
    }
}

struct PhpDocTypesVisitor<'s> {
    source: &'s str,
    file_path: PathBuf,
    builtin_classes: &'s [&'static str],
    symbol_table: Option<&'s SymbolTable>,
    current_namespace: String,
    defined_classes: HashSet<String>,
    imported_classes: HashSet<String>,
    use_fqn_map: HashMap<String, String>,
    template_params: HashSet<String>,
    issues: Vec<Issue>,
}

impl<'s> PhpDocTypesVisitor<'s> {
    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_definitions_in_stmt(stmt, "");
        }
    }

    fn collect_definitions_in_stmt<'a>(&mut self, stmt: &Statement<'a>, namespace: &str) {
        match stmt {
            Statement::Class(class) => {
                let name = &self.source[class.name.span.start.offset as usize..class.name.span.end.offset as usize];
                let fqn = if namespace.is_empty() {
                    name.to_string()
                } else {
                    format!("{}\\{}", namespace, name)
                };
                self.defined_classes.insert(name.to_lowercase());
                self.defined_classes.insert(fqn.to_lowercase());
            }
            Statement::Interface(iface) => {
                let name = &self.source[iface.name.span.start.offset as usize..iface.name.span.end.offset as usize];
                let fqn = if namespace.is_empty() {
                    name.to_string()
                } else {
                    format!("{}\\{}", namespace, name)
                };
                self.defined_classes.insert(name.to_lowercase());
                self.defined_classes.insert(fqn.to_lowercase());
            }
            Statement::Trait(tr) => {
                let name = &self.source[tr.name.span.start.offset as usize..tr.name.span.end.offset as usize];
                let fqn = if namespace.is_empty() {
                    name.to_string()
                } else {
                    format!("{}\\{}", namespace, name)
                };
                self.defined_classes.insert(name.to_lowercase());
                self.defined_classes.insert(fqn.to_lowercase());
            }
            Statement::Enum(en) => {
                let name = &self.source[en.name.span.start.offset as usize..en.name.span.end.offset as usize];
                let fqn = if namespace.is_empty() {
                    name.to_string()
                } else {
                    format!("{}\\{}", namespace, name)
                };
                self.defined_classes.insert(name.to_lowercase());
                self.defined_classes.insert(fqn.to_lowercase());
            }
            Statement::Use(use_stmt) => {
                self.collect_use_imports(use_stmt);
            }
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
                            self.collect_definitions_in_stmt(inner, &ns_name);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.collect_definitions_in_stmt(inner, &ns_name);
                        }
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_definitions_in_stmt(inner, namespace);
                }
            }
            _ => {}
        }
    }

    fn collect_use_imports<'a>(&mut self, use_stmt: &Use<'a>) {
        let use_span = use_stmt.span();
        let use_text = &self.source[use_span.start.offset as usize..use_span.end.offset as usize];
        self.extract_imports_from_use_text(use_text);
    }

    fn extract_imports_from_use_text(&mut self, use_text: &str) {
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
                        self.imported_classes.insert(alias.to_lowercase());
                        self.use_fqn_map.insert(alias.to_lowercase(), fqn);
                    } else {
                        let name = item.rsplit('\\').next().unwrap_or(item).trim();
                        if !name.is_empty() {
                            let fqn = format!("{}\\{}", prefix, item.trim());
                            self.imported_classes.insert(name.to_lowercase());
                            self.use_fqn_map.insert(name.to_lowercase(), fqn);
                        }
                    }
                }
            }
        } else {
            if let Some(as_pos) = text.to_lowercase().find(" as ") {
                let fqn = text[..as_pos].trim().to_string();
                let alias = text[as_pos + 4..].trim();
                self.imported_classes.insert(alias.to_lowercase());
                self.use_fqn_map.insert(alias.to_lowercase(), fqn);
            } else {
                let name = text.rsplit('\\').next().unwrap_or(text).trim();
                if !name.is_empty() {
                    self.imported_classes.insert(name.to_lowercase());
                    self.use_fqn_map.insert(name.to_lowercase(), text.to_string());
                }
            }
        }
    }

    fn validate_phpdocs<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.validate_stmt_phpdocs(stmt);
        }
    }

    fn validate_stmt_phpdocs<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                // Collect @template params for this function
                let phpdoc = self.extract_phpdoc(func.span().start.offset as usize);
                if let Some(ref doc) = phpdoc {
                    for template in &doc.templates {
                        self.template_params.insert(template.name.to_lowercase());
                    }
                    self.validate_phpdoc(doc, func.span().start.offset as usize);
                }
                self.template_params.clear();
            }
            Statement::Class(class) => {
                // Collect class-level @template params
                let class_phpdoc = self.extract_phpdoc(class.span().start.offset as usize);
                if let Some(ref doc) = class_phpdoc {
                    for template in &doc.templates {
                        self.template_params.insert(template.name.to_lowercase());
                    }
                    self.validate_phpdoc(doc, class.span().start.offset as usize);
                }

                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Property(prop) => {
                            let phpdoc = self.extract_phpdoc(prop.span().start.offset as usize);
                            if let Some(ref doc) = phpdoc {
                                self.validate_phpdoc(doc, prop.span().start.offset as usize);
                            }
                        }
                        ClassLikeMember::Method(method) => {
                            // Collect method-level @template params
                            let method_phpdoc = self.extract_phpdoc(method.span().start.offset as usize);
                            let saved_templates = self.template_params.clone();

                            if let Some(ref doc) = method_phpdoc {
                                for template in &doc.templates {
                                    self.template_params.insert(template.name.to_lowercase());
                                }
                                self.validate_phpdoc(doc, method.span().start.offset as usize);
                            }

                            self.template_params = saved_templates;
                        }
                        ClassLikeMember::Constant(constant) => {
                            let phpdoc = self.extract_phpdoc(constant.span().start.offset as usize);
                            if let Some(ref doc) = phpdoc {
                                self.validate_phpdoc(doc, constant.span().start.offset as usize);
                            }
                        }
                        _ => {}
                    }
                }

                self.template_params.clear();
            }
            Statement::Interface(iface) => {
                let iface_phpdoc = self.extract_phpdoc(iface.span().start.offset as usize);
                if let Some(ref doc) = iface_phpdoc {
                    for template in &doc.templates {
                        self.template_params.insert(template.name.to_lowercase());
                    }
                    self.validate_phpdoc(doc, iface.span().start.offset as usize);
                }

                for member in iface.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_phpdoc = self.extract_phpdoc(method.span().start.offset as usize);
                        let saved_templates = self.template_params.clone();

                        if let Some(ref doc) = method_phpdoc {
                            for template in &doc.templates {
                                self.template_params.insert(template.name.to_lowercase());
                            }
                            self.validate_phpdoc(doc, method.span().start.offset as usize);
                        }

                        self.template_params = saved_templates;
                    }
                }

                self.template_params.clear();
            }
            Statement::Trait(tr) => {
                let trait_phpdoc = self.extract_phpdoc(tr.span().start.offset as usize);
                if let Some(ref doc) = trait_phpdoc {
                    for template in &doc.templates {
                        self.template_params.insert(template.name.to_lowercase());
                    }
                    self.validate_phpdoc(doc, tr.span().start.offset as usize);
                }

                for member in tr.members.iter() {
                    match member {
                        ClassLikeMember::Property(prop) => {
                            let phpdoc = self.extract_phpdoc(prop.span().start.offset as usize);
                            if let Some(ref doc) = phpdoc {
                                self.validate_phpdoc(doc, prop.span().start.offset as usize);
                            }
                        }
                        ClassLikeMember::Method(method) => {
                            let method_phpdoc = self.extract_phpdoc(method.span().start.offset as usize);
                            let saved_templates = self.template_params.clone();

                            if let Some(ref doc) = method_phpdoc {
                                for template in &doc.templates {
                                    self.template_params.insert(template.name.to_lowercase());
                                }
                                self.validate_phpdoc(doc, method.span().start.offset as usize);
                            }

                            self.template_params = saved_templates;
                        }
                        _ => {}
                    }
                }

                self.template_params.clear();
            }
            Statement::Namespace(ns) => {
                match &ns.body {
                    NamespaceBody::Implicit(body) => {
                        for inner in body.statements.iter() {
                            self.validate_stmt_phpdocs(inner);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.validate_stmt_phpdocs(inner);
                        }
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.validate_stmt_phpdocs(inner);
                }
            }
            _ => {}
        }
    }

    fn extract_phpdoc(&self, offset: usize) -> Option<PhpDoc> {
        let before = &self.source[..offset];

        if let Some(doc_end) = before.rfind("*/") {
            if let Some(doc_start) = before[..doc_end].rfind("/**") {
                let between = &before[doc_end + 2..];
                let between_trimmed = between.trim();

                let is_valid = between_trimmed.is_empty()
                    || between_trimmed.chars().all(|c| c.is_whitespace())
                    || between_trimmed.starts_with('#')
                    || between_trimmed.starts_with("public")
                    || between_trimmed.starts_with("private")
                    || between_trimmed.starts_with("protected")
                    || between_trimmed.starts_with("static")
                    || between_trimmed.starts_with("final")
                    || between_trimmed.starts_with("abstract")
                    || between_trimmed.starts_with("readonly");

                if is_valid {
                    let doc_comment = &self.source[doc_start..doc_end + 2];
                    return Some(parse_phpdoc(doc_comment));
                }
            }
        }
        None
    }

    fn validate_phpdoc(&mut self, doc: &PhpDoc, offset: usize) {
        // Validate @param types
        for (_, param_type) in &doc.params {
            self.validate_type(param_type, offset);
        }

        // Validate @return type
        if let Some(ref return_type) = doc.return_type {
            self.validate_type(return_type, offset);
        }

        // Validate @var type
        if let Some(ref var_type) = doc.var_type {
            self.validate_type(var_type, offset);
        }

        // Validate @throws types
        for throws_type in &doc.throws {
            self.validate_type(throws_type, offset);
        }

        // Validate @property types
        for (_, prop_type, _) in &doc.properties {
            self.validate_type(prop_type, offset);
        }

        // Validate @method types
        for method_sig in &doc.methods {
            self.validate_type(&method_sig.return_type, offset);
            for (_, param_type) in &method_sig.params {
                self.validate_type(param_type, offset);
            }
        }
    }

    fn validate_type(&mut self, ty: &Type, offset: usize) {
        // Extract all class names from the type and validate each
        let class_names = self.extract_class_names(ty);

        for class_name in class_names {
            // Clean up malformed class names (trailing backslash, etc.)
            let cleaned_name = class_name.trim_end_matches('\\');
            if cleaned_name.is_empty() {
                continue;
            }

            // Skip constant references like Country::MEXICO (PHPDoc literal value syntax)
            // These have :: followed by UPPERCASE, which is a class constant, not a class
            if let Some(double_colon_pos) = cleaned_name.find("::") {
                let after = &cleaned_name[double_colon_pos + 2..];
                // If what follows :: is uppercase or a constant-like name, skip
                if after.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    continue;
                }
            }

            if !self.is_class_defined(cleaned_name) {
                let (line, col) = self.get_line_col(offset);
                self.issues.push(
                    Issue::error(
                        "class.notFound",
                        format!("Class {} not found.", cleaned_name),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("class.notFound"),
                );
            }
        }
    }

    fn extract_class_names(&self, ty: &Type) -> Vec<String> {
        let mut names = Vec::new();
        self.collect_class_names(ty, &mut names);
        names
    }

    fn collect_class_names(&self, ty: &Type, names: &mut Vec<String>) {
        match ty {
            Type::Object { class_name: Some(name) } => {
                names.push(name.clone());
            }
            Type::ClassString { class_name: Some(name) } => {
                names.push(name.clone());
            }
            Type::Array { key, value } => {
                self.collect_class_names(key, names);
                self.collect_class_names(value, names);
            }
            Type::List { value } => {
                self.collect_class_names(value, names);
            }
            Type::NonEmptyArray { key, value } => {
                self.collect_class_names(key, names);
                self.collect_class_names(value, names);
            }
            Type::Iterable { key, value } => {
                self.collect_class_names(key, names);
                self.collect_class_names(value, names);
            }
            Type::Union(types) => {
                for t in types {
                    self.collect_class_names(t, names);
                }
            }
            Type::Intersection(types) => {
                for t in types {
                    self.collect_class_names(t, names);
                }
            }
            Type::Nullable(inner) => {
                self.collect_class_names(inner, names);
            }
            Type::Template { bound, .. } => {
                if let Some(b) = bound {
                    self.collect_class_names(b, names);
                }
            }
            _ => {}
        }
    }

    fn is_class_defined(&self, name: &str) -> bool {
        let lower_name = name.to_lowercase();

        // Skip special types
        if matches!(
            lower_name.as_str(),
            "self" | "static" | "parent" | "$this" | "this"
        ) {
            return true;
        }

        // Check if it's a template parameter
        if self.template_params.contains(&lower_name) {
            return true;
        }

        // Check PHP builtin classes
        if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(name)) {
            return true;
        }

        // Check user-defined classes in this file
        if self.defined_classes.contains(&lower_name) {
            return true;
        }

        // Handle absolute FQN (starts with backslash)
        let is_absolute_fqn = name.starts_with('\\');
        let has_namespace = name.contains('\\');

        if is_absolute_fqn {
            if let Some(symbol_table) = self.symbol_table {
                let normalized = name.trim_start_matches('\\');
                return symbol_table.get_class(normalized).is_some();
            }
            return false;
        }

        if has_namespace {
            // Relative namespace - resolve alias first
            let first_part = name.split('\\').next().unwrap_or(name);
            let first_part_lower = first_part.to_lowercase();

            if let Some(fqn_prefix) = self.use_fqn_map.get(&first_part_lower) {
                let rest = &name[first_part.len()..];
                let resolved_fqn = format!("{}{}", fqn_prefix, rest);

                if let Some(symbol_table) = self.symbol_table {
                    if symbol_table.get_class(&resolved_fqn).is_some() {
                        return true;
                    }
                } else {
                    return true;
                }
            }

            // Try looking up as-is in symbol table
            if let Some(symbol_table) = self.symbol_table {
                if symbol_table.get_class(name).is_some() {
                    return true;
                }
                if !self.current_namespace.is_empty() {
                    let fqn = format!("{}\\{}", self.current_namespace, name);
                    if symbol_table.get_class(&fqn).is_some() {
                        return true;
                    }
                }
            }

            return false;
        }

        // Simple name - check imports and symbol table
        if self.imported_classes.contains(&lower_name) {
            if let Some(symbol_table) = self.symbol_table {
                if let Some(fqn) = self.use_fqn_map.get(&lower_name) {
                    if symbol_table.get_class(fqn).is_some() {
                        return true;
                    }
                    let fqn_no_leading = fqn.trim_start_matches('\\');
                    if symbol_table.get_class(fqn_no_leading).is_some() {
                        return true;
                    }
                }
                return false;
            } else {
                return true;
            }
        }

        // Check symbol table directly
        if let Some(symbol_table) = self.symbol_table {
            if symbol_table.get_class(name).is_some() {
                return true;
            }
            if !self.current_namespace.is_empty() {
                let fqn = format!("{}\\{}", self.current_namespace, name);
                if symbol_table.get_class(&fqn).is_some() {
                    return true;
                }
            }
        }

        false
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phpdoc_types_check_level() {
        let check = PhpDocTypesCheck;
        assert_eq!(check.level(), 0);  // PHPStan reports PHPDoc class.notFound at level 0
    }

    #[test]
    fn test_extract_class_names_simple() {
        let visitor = PhpDocTypesVisitor {
            source: "",
            file_path: PathBuf::new(),
            builtin_classes: &[],
            symbol_table: None,
            current_namespace: String::new(),
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
            use_fqn_map: HashMap::new(),
            template_params: HashSet::new(),
            issues: Vec::new(),
        };

        let ty = Type::Object {
            class_name: Some("MyClass".to_string()),
        };
        let names = visitor.extract_class_names(&ty);
        assert_eq!(names, vec!["MyClass"]);
    }

    #[test]
    fn test_extract_class_names_union() {
        let visitor = PhpDocTypesVisitor {
            source: "",
            file_path: PathBuf::new(),
            builtin_classes: &[],
            symbol_table: None,
            current_namespace: String::new(),
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
            use_fqn_map: HashMap::new(),
            template_params: HashSet::new(),
            issues: Vec::new(),
        };

        let ty = Type::Union(vec![
            Type::Object {
                class_name: Some("ClassA".to_string()),
            },
            Type::Object {
                class_name: Some("ClassB".to_string()),
            },
        ]);
        let names = visitor.extract_class_names(&ty);
        assert_eq!(names, vec!["ClassA", "ClassB"]);
    }

    #[test]
    fn test_extract_class_names_array() {
        let visitor = PhpDocTypesVisitor {
            source: "",
            file_path: PathBuf::new(),
            builtin_classes: &[],
            symbol_table: None,
            current_namespace: String::new(),
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
            use_fqn_map: HashMap::new(),
            template_params: HashSet::new(),
            issues: Vec::new(),
        };

        let ty = Type::Array {
            key: Box::new(Type::String),
            value: Box::new(Type::Object {
                class_name: Some("MyClass".to_string()),
            }),
        };
        let names = visitor.extract_class_names(&ty);
        assert_eq!(names, vec!["MyClass"]);
    }
}
