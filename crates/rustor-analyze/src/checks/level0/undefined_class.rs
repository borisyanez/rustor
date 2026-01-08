//! Check for references to undefined classes

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::HashSet;

pub struct UndefinedClassCheck;

impl Check for UndefinedClassCheck {
    fn id(&self) -> &'static str {
        "undefined.class"
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
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
            issues: Vec::new(),
        };

        // First pass: collect class definitions and use imports
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
    defined_classes: HashSet<String>,
    imported_classes: HashSet<String>,
    issues: Vec<Issue>,
}

impl<'s> UndefinedClassVisitor<'s> {
    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_definitions_in_stmt(stmt);
        }
    }

    fn collect_definitions_in_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                let name = &self.source[class.name.span.start.offset as usize..class.name.span.end.offset as usize];
                self.defined_classes.insert(name.to_lowercase());
            }
            Statement::Interface(iface) => {
                let name = &self.source[iface.name.span.start.offset as usize..iface.name.span.end.offset as usize];
                self.defined_classes.insert(name.to_lowercase());
            }
            Statement::Trait(tr) => {
                let name = &self.source[tr.name.span.start.offset as usize..tr.name.span.end.offset as usize];
                self.defined_classes.insert(name.to_lowercase());
            }
            Statement::Enum(en) => {
                let name = &self.source[en.name.span.start.offset as usize..en.name.span.end.offset as usize];
                self.defined_classes.insert(name.to_lowercase());
            }
            // Collect use imports
            Statement::Use(use_stmt) => {
                self.collect_use_imports(use_stmt);
            }
            Statement::Namespace(ns) => {
                match &ns.body {
                    NamespaceBody::Implicit(body) => {
                        for inner in body.statements.iter() {
                            self.collect_definitions_in_stmt(inner);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.collect_definitions_in_stmt(inner);
                        }
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_definitions_in_stmt(inner);
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
            if let Some(brace_end) = text.find('}') {
                let group_content = &text[brace_start + 1..brace_end];
                for item in group_content.split(',') {
                    let item = item.trim();
                    // Handle "Bar as Baz" - use alias
                    if let Some(as_pos) = item.to_lowercase().find(" as ") {
                        let alias = item[as_pos + 4..].trim();
                        self.imported_classes.insert(alias.to_lowercase());
                    } else {
                        // Just "Bar" - use last segment
                        let name = item.rsplit('\\').next().unwrap_or(item).trim();
                        if !name.is_empty() {
                            self.imported_classes.insert(name.to_lowercase());
                        }
                    }
                }
            }
        } else {
            // Simple import: Foo\Bar or Foo\Bar as Baz
            // Handle "as" alias
            if let Some(as_pos) = text.to_lowercase().find(" as ") {
                let alias = text[as_pos + 4..].trim();
                self.imported_classes.insert(alias.to_lowercase());
            } else {
                // Get last segment of namespace
                let name = text.rsplit('\\').next().unwrap_or(text).trim();
                if !name.is_empty() {
                    self.imported_classes.insert(name.to_lowercase());
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

        // Check user-defined classes
        if self.defined_classes.contains(&lower_name) {
            return true;
        }

        // Check imported classes (from use statements)
        if self.imported_classes.contains(&lower_name) {
            return true;
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
        // Skip fully qualified names (they reference classes we can't resolve)
        if name.starts_with('\\') || name.contains('\\') {
            return;
        }

        // Skip special class names
        if matches!(name.to_lowercase().as_str(), "self" | "static" | "parent") {
            return;
        }

        if !self.is_defined(name) {
            let (line, col) = self.get_line_col(offset);
            self.issues.push(
                Issue::error(
                    "undefined.class",
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
                if let Expression::Identifier(ident) = &*call.class {
                    let span = ident.span();
                    let name = &self.source[span.start.offset as usize..span.end.offset as usize];
                    self.check_class_name(name, span.start.offset as usize);
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
                    // Skip ClassName::class syntax
                    let const_span = access.constant.span();
                    let const_name = &self.source[const_span.start.offset as usize..const_span.end.offset as usize];
                    if !const_name.eq_ignore_ascii_case("class") {
                        self.check_class_name(name, span.start.offset as usize);
                    }
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
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
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
            defined_classes: HashSet::new(),
            imported_classes: HashSet::new(),
            issues: Vec::new(),
        };

        // Test simple use
        visitor.extract_imports_from_use_text("use Foo\\Bar;");
        assert!(visitor.imported_classes.contains("bar"));

        // Test use with alias
        visitor.extract_imports_from_use_text("use Foo\\Bar as Baz;");
        assert!(visitor.imported_classes.contains("baz"));

        // Test grouped use
        visitor.extract_imports_from_use_text("use Foo\\{Bar, Qux};");
        assert!(visitor.imported_classes.contains("bar"));
        assert!(visitor.imported_classes.contains("qux"));
    }
}
