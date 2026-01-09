//! Check for access to undefined class constants (Level 0)

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::{HashMap, HashSet};

/// Checks for class constant access like Foo::CONSTANT
pub struct ClassConstantCheck;

impl Check for ClassConstantCheck {
    fn id(&self) -> &'static str {
        "classConstant.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects access to undefined class constants"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = ClassConstantVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_constants: HashMap::new(),
            builtin_classes: ctx.builtin_classes,
            issues: Vec::new(),
        };

        // First pass: collect class constant definitions
        visitor.collect_definitions(program);

        // Second pass: check constant access
        visitor.visit_program(program, ctx.source);

        visitor.issues
    }
}

struct ClassConstantVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    class_constants: HashMap<String, HashSet<String>>, // class name -> constant names
    builtin_classes: &'s [&'static str],
    issues: Vec<Issue>,
}

impl<'s> ClassConstantVisitor<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_from_stmt(stmt);
        }
    }

    fn collect_from_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_lowercase();
                let mut constants = HashSet::new();

                // Always add 'class' constant (available on all classes)
                constants.insert("class".to_string());

                // Collect constants from class members
                for member in class.members.iter() {
                    if let ClassLikeMember::Constant(const_member) = member {
                        for item in const_member.items.iter() {
                            let const_name = self.get_span_text(&item.name.span).to_lowercase();
                            constants.insert(const_name);
                        }
                    }
                }

                self.class_constants.insert(class_name, constants);
            }
            Statement::Interface(interface) => {
                let interface_name = self.get_span_text(&interface.name.span).to_lowercase();
                let mut constants = HashSet::new();

                constants.insert("class".to_string());

                for member in interface.members.iter() {
                    if let ClassLikeMember::Constant(const_member) = member {
                        for item in const_member.items.iter() {
                            let const_name = self.get_span_text(&item.name.span).to_lowercase();
                            constants.insert(const_name);
                        }
                    }
                }

                self.class_constants.insert(interface_name, constants);
            }
            Statement::Enum(enum_def) => {
                let enum_name = self.get_span_text(&enum_def.name.span).to_lowercase();
                let mut constants = HashSet::new();

                constants.insert("class".to_string());

                // Enum cases are also accessible as constants
                for member in enum_def.members.iter() {
                    match member {
                        ClassLikeMember::EnumCase(case) => {
                            let case_name = self.get_span_text(&case.item.name().span).to_lowercase();
                            constants.insert(case_name);
                        }
                        ClassLikeMember::Constant(const_member) => {
                            for item in const_member.items.iter() {
                                let const_name = self.get_span_text(&item.name.span).to_lowercase();
                                constants.insert(const_name);
                            }
                        }
                        _ => {}
                    }
                }

                self.class_constants.insert(enum_name, constants);
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
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_from_stmt(inner);
                }
            }
            _ => {}
        }
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

impl<'a, 's> Visitor<'a> for ClassConstantVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Check for Class::CONSTANT access
        if let Expression::Access(Access::ClassConstant(const_access)) = expr {
            // Get class name
            let class_name = match &*const_access.class {
                Expression::Identifier(ident) => {
                    Some(self.get_span_text(&ident.span()).to_string())
                }
                _ => None,
            };

            // Get constant name
            let const_info = match &const_access.constant {
                ClassLikeConstantSelector::Identifier(ident) => {
                    Some((self.get_span_text(&ident.span).to_string(), ident.span))
                }
                _ => None,
            };

            if let (Some(class), Some((constant, const_span))) = (class_name, const_info) {
                // Skip self, static, parent
                if class.eq_ignore_ascii_case("self")
                    || class.eq_ignore_ascii_case("static")
                    || class.eq_ignore_ascii_case("parent")
                {
                    return true;
                }

                // Skip built-in classes
                if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&class)) {
                    return true;
                }

                let class_lower = class.to_lowercase();
                let const_lower = constant.to_lowercase();

                // Check if we have this class defined and if it has the constant
                if let Some(constants) = self.class_constants.get(&class_lower) {
                    if !constants.contains(&const_lower) {
                        let (line, col) = self.get_line_col(const_span.start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "classConstant.notFound",
                                format!("Access to undefined constant {}::{}.", class, constant),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("classConstant.notFound"),
                        );
                    }
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
    fn test_class_constant_check_level() {
        let check = ClassConstantCheck;
        assert_eq!(check.level(), 0);
    }
}
