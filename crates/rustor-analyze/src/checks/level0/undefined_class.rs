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
            issues: Vec::new(),
        };

        // First pass: collect class definitions
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
            issues: Vec::new(),
        };

        // These should not generate issues (handled by check_class_name skipping them)
        assert!(!visitor.is_defined("self"));  // is_defined returns false, but check_class_name skips it
        assert!(!visitor.is_defined("static"));
        assert!(!visitor.is_defined("parent"));
    }
}
