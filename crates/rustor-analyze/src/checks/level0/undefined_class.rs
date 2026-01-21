//! Check for references to undefined classes

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::HashSet;

pub struct UndefinedClassCheck;

impl Check for UndefinedClassCheck {
    fn id(&self) -> &'static str {
        "class.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects references to undefined classes"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = UndefinedClassVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            builtin_classes: ctx.builtin_classes,
            symbol_table: ctx.symbol_table,
            current_namespace: String::new(),
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
            use_fqn_map: std::collections::HashMap::new(),
            issues: Vec::new(),
        };

        // First pass: collect class definitions, imports, and namespace
        visitor.collect_definitions(program);

        // Second pass: check class references
        visitor.visit_program(program, ctx.source);

        visitor.issues
    }
}

struct UndefinedClassVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    builtin_classes: &'s [&'static str],
    symbol_table: Option<&'s SymbolTable>,
    current_namespace: String,
    defined_classes: HashSet<String>,
    imported_classes: HashSet<String>,
    use_fqn_map: std::collections::HashMap<String, String>, // short name -> FQN
    issues: Vec<Issue>,
}

impl<'s> UndefinedClassVisitor<'s> {
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
            // Collect use imports
            Statement::Use(use_stmt) => {
                self.collect_use_imports(use_stmt);
            }
            Statement::Namespace(ns) => {
                // Extract namespace name
                let ns_name = if let Some(ref name) = ns.name {
                    let span = name.span();
                    self.source[span.start.offset as usize..span.end.offset as usize].to_string()
                } else {
                    String::new()
                };

                // Set current namespace for FQN resolution
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
        // Get the full use statement text and parse imported class names
        let use_span = use_stmt.span();
        let use_text = &self.source[use_span.start.offset as usize..use_span.end.offset as usize];

        // Extract class names from use statement
        // Handles: use Foo\Bar; use Foo\Bar as Baz; use Foo\{Bar, Baz};
        self.extract_imports_from_use_text(use_text);
    }

    fn extract_imports_from_use_text(&mut self, use_text: &str) {
        // Remove 'use', 'function', 'const' keywords and semicolon
        let text = use_text
            .trim_start_matches("use")
            .trim_start()
            .trim_start_matches("function")
            .trim_start_matches("const")
            .trim()
            .trim_end_matches(';')
            .trim();

        // Handle grouped imports: Foo\{Bar, Baz as Qux}
        if let Some(brace_start) = text.find('{') {
            let prefix = text[..brace_start].trim().trim_end_matches('\\');
            if let Some(brace_end) = text.find('}') {
                let group_content = &text[brace_start + 1..brace_end];
                for item in group_content.split(',') {
                    let item = item.trim();
                    // Handle "Bar as Baz" - use alias
                    if let Some(as_pos) = item.to_lowercase().find(" as ") {
                        let class_part = item[..as_pos].trim();
                        let alias = item[as_pos + 4..].trim();
                        let fqn = format!("{}\\{}", prefix, class_part);
                        self.imported_classes.insert(alias.to_lowercase());
                        self.use_fqn_map.insert(alias.to_lowercase(), fqn);
                    } else {
                        // Just "Bar" - use last segment
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
            // Simple import: Foo\Bar or Foo\Bar as Baz
            // Handle "as" alias
            if let Some(as_pos) = text.to_lowercase().find(" as ") {
                let fqn = text[..as_pos].trim().to_string();
                let alias = text[as_pos + 4..].trim();
                self.imported_classes.insert(alias.to_lowercase());
                self.use_fqn_map.insert(alias.to_lowercase(), fqn);
            } else {
                // Get last segment of namespace
                let name = text.rsplit('\\').next().unwrap_or(text).trim();
                if !name.is_empty() {
                    self.imported_classes.insert(name.to_lowercase());
                    self.use_fqn_map.insert(name.to_lowercase(), text.to_string());
                }
            }
        }
    }

    fn is_defined(&self, name: &str) -> bool {
        let lower_name = name.to_lowercase();

        // Check builtin classes (case-insensitive)
        if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(name)) {
            return true;
        }

        // Check user-defined classes in this file
        if self.defined_classes.contains(&lower_name) {
            return true;
        }

        // Check if imported and verify it exists in symbol table
        if self.imported_classes.contains(&lower_name) {
            // If we have a symbol table, verify the imported class exists
            if let Some(symbol_table) = self.symbol_table {
                if let Some(fqn) = self.use_fqn_map.get(&lower_name) {
                    // Check if the FQN exists in symbol table
                    if symbol_table.get_class(fqn).is_some() {
                        return true;
                    }
                    // Also try without leading backslash
                    let fqn_no_leading = fqn.trim_start_matches('\\');
                    if symbol_table.get_class(fqn_no_leading).is_some() {
                        return true;
                    }
                }
                // Import exists but class not found in symbol table
                return false;
            } else {
                // No symbol table - trust the import (backwards compatible)
                return true;
            }
        }

        // Check in symbol table directly (for FQN references or same-namespace classes)
        if let Some(symbol_table) = self.symbol_table {
            // Try the name as-is
            if symbol_table.get_class(name).is_some() {
                return true;
            }
            // Try with current namespace prefix
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

    fn check_class_name(&mut self, name: &str, offset: usize) {
        // Skip special class names
        if matches!(name.to_lowercase().as_str(), "self" | "static" | "parent") {
            return;
        }

        // Handle fully qualified names (with backslash)
        let is_fqn = name.starts_with('\\') || name.contains('\\');

        if is_fqn {
            // For FQN, only check if we have a symbol table (from autoloader)
            if let Some(symbol_table) = self.symbol_table {
                // Normalize: remove leading backslash for lookup
                let normalized = name.trim_start_matches('\\');

                // Check if class exists in symbol table
                if symbol_table.get_class(normalized).is_none() {
                    let (line, col) = self.get_line_col(offset);
                    self.issues.push(
                        Issue::error(
                            "class.notFound",
                            format!("Class {} not found", name),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("class.notFound"),
                    );
                }
            }
            // If no symbol table, skip FQN checks (can't verify without autoloader)
            return;
        }

        // For non-FQN, use existing resolution logic
        if !self.is_defined(name) {
            let (line, col) = self.get_line_col(offset);
            self.issues.push(
                Issue::error(
                    "class.notFound",
                    format!("Class {} not found", name),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("class.notFound"),
            );
        }
    }
}

impl<'a, 's> Visitor<'a> for UndefinedClassVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        match expr {
            // Check new ClassName()
            Expression::Instantiation(instantiate) => {
                if let Expression::Identifier(ident) = &*instantiate.class {
                    let span = ident.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.check_class_name(name, span.start.offset as usize);
                }
            }
            // Check ClassName::method() or ClassName::$property
            Expression::Call(Call::StaticMethod(call)) => {
                // Get the class name from the span, regardless of expression type
                let span = call.class.span();
                let class_name = &self.source[span.start.offset as usize..span.end.offset as usize];
                // Skip complex expressions like $var::method()
                if !class_name.contains('$') && !class_name.contains('(') {
                    self.check_class_name(class_name, span.start.offset as usize);
                }
            }
            Expression::Access(Access::StaticProperty(access)) => {
                if let Expression::Identifier(ident) = &*access.class {
                    let span = ident.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.check_class_name(name, span.start.offset as usize);
                }
            }
            Expression::Access(Access::ClassConstant(access)) => {
                if let Expression::Identifier(ident) = &*access.class {
                    let span = ident.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    // Check all class constant access including ::class syntax
                    self.check_class_name(name, span.start.offset as usize);
                }
            }
            _ => {}
        }
        true
    }

    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        // Check extends/implements
        if let Statement::Class(class) = stmt {
            // Check extends
            if let Some(extends) = &class.extends {
                for parent in extends.types.iter() {
                    let span = parent.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.check_class_name(name, span.start.offset as usize);
                }
            }
            // Check implements
            if let Some(implements) = &class.implements {
                for iface in implements.types.iter() {
                    let span = iface.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.check_class_name(name, span.start.offset as usize);
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
    fn test_check_class_name_special() {
        let visitor = UndefinedClassVisitor {
            source: "",
            file_path: std::path::PathBuf::new(),
            builtin_classes: &[],
            symbol_table: None,
            current_namespace: String::new(),
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
            use_fqn_map: std::collections::HashMap::new(),
            issues: Vec::new(),
        };

        // These should not generate issues (handled by check_class_name skipping them)
        assert!(!visitor.is_defined("self"));  // is_defined returns false, but check_class_name skips it
        assert!(!visitor.is_defined("static"));
        assert!(!visitor.is_defined("parent"));
    }

    #[test]
    fn test_extract_imports() {
        let mut visitor = UndefinedClassVisitor {
            source: "",
            file_path: std::path::PathBuf::new(),
            builtin_classes: &[],
            symbol_table: None,
            current_namespace: String::new(),
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
            use_fqn_map: std::collections::HashMap::new(),
            issues: Vec::new(),
        };

        // Test simple use
        visitor.extract_imports_from_use_text("use Foo\\Bar;");
        assert!(visitor.imported_classes.contains("bar"));
        assert_eq!(visitor.use_fqn_map.get("bar"), Some(&"Foo\\Bar".to_string()));

        // Test use with alias
        visitor.extract_imports_from_use_text("use Foo\\Bar as Baz;");
        assert!(visitor.imported_classes.contains("baz"));
        assert_eq!(visitor.use_fqn_map.get("baz"), Some(&"Foo\\Bar".to_string()));

        // Test grouped use
        visitor.extract_imports_from_use_text("use Foo\\{Bar, Qux};");
        assert!(visitor.imported_classes.contains("bar"));
        assert!(visitor.imported_classes.contains("qux"));
        assert_eq!(visitor.use_fqn_map.get("bar"), Some(&"Foo\\Bar".to_string()));
        assert_eq!(visitor.use_fqn_map.get("qux"), Some(&"Foo\\Qux".to_string()));
    }
}
