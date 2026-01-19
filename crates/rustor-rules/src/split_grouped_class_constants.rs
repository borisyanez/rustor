//! Rule: split_grouped_class_constants
//!
//! Splits grouped class constant declarations into separate statements.
//!
//! Pattern:
//! ```php
//! // Before
//! class Foo {
//!     const A = 1, B = 2;
//! }
//!
//! // After
//! class Foo {
//!     const A = 1;
//!     const B = 2;
//! }
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_split_grouped_class_constants<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut checker = SplitGroupedClassConstantsChecker {
        source,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct SplitGroupedClassConstantsChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SplitGroupedClassConstantsChecker<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn check_program(&mut self, program: &Program<'_>) {
        for stmt in program.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::Class(class) => {
                self.check_class(class);
            }
            Statement::Interface(iface) => {
                self.check_interface(iface);
            }
            Statement::Trait(trait_def) => {
                self.check_trait(trait_def);
            }
            Statement::Enum(enum_def) => {
                self.check_enum(enum_def);
            }
            Statement::Namespace(ns) => {
                let statements = match &ns.body {
                    NamespaceBody::Implicit(body) => &body.statements,
                    NamespaceBody::BraceDelimited(body) => &body.statements,
                };
                for inner in statements.iter() {
                    self.check_statement(inner);
                }
            }
            _ => {}
        }
    }

    fn check_class(&mut self, class: &Class<'_>) {
        for member in class.members.iter() {
            if let ClassLikeMember::Constant(const_member) = member {
                self.check_class_constant(const_member);
            }
        }
    }

    fn check_interface(&mut self, iface: &Interface<'_>) {
        for member in iface.members.iter() {
            if let ClassLikeMember::Constant(const_member) = member {
                self.check_class_constant(const_member);
            }
        }
    }

    fn check_trait(&mut self, trait_def: &Trait<'_>) {
        for member in trait_def.members.iter() {
            if let ClassLikeMember::Constant(const_member) = member {
                self.check_class_constant(const_member);
            }
        }
    }

    fn check_enum(&mut self, enum_def: &Enum<'_>) {
        for member in enum_def.members.iter() {
            if let ClassLikeMember::Constant(const_member) = member {
                self.check_class_constant(const_member);
            }
        }
    }

    fn check_class_constant(&mut self, const_member: &ClassLikeConstant<'_>) {
        // Only process if there are multiple constants in one declaration
        if const_member.items.len() < 2 {
            return;
        }

        // Build modifiers string
        let mut modifiers = String::new();
        for modifier in const_member.modifiers.iter() {
            modifiers.push_str(self.get_text(modifier.span()));
            modifiers.push(' ');
        }

        // Build type hint if present
        let type_hint = const_member
            .hint
            .as_ref()
            .map(|h| format!("{} ", self.get_text(h.span())))
            .unwrap_or_default();

        // Build separate constant declarations
        let mut declarations = Vec::new();
        for item in const_member.items.iter() {
            let name = self.get_text(item.name.span());
            let value = self.get_text(item.value.span());
            declarations.push(format!("{}const {}{} = {}", modifiers, type_hint, name, value));
        }

        let replacement = declarations.join(";\n    ");

        self.edits.push(Edit::new(
            const_member.span(),
            format!("{};", replacement),
            "Split grouped class constants into separate declarations".to_string(),
        ));
    }
}

pub struct SplitGroupedClassConstantsRule;

impl SplitGroupedClassConstantsRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SplitGroupedClassConstantsRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for SplitGroupedClassConstantsRule {
    fn name(&self) -> &'static str {
        "split_grouped_class_constants"
    }

    fn description(&self) -> &'static str {
        "Split grouped class constant declarations into separate statements"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_split_grouped_class_constants(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_split_grouped_class_constants(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_simple_grouped_constants() {
        let source = r#"<?php
class Foo {
    const A = 1, B = 2;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("const A = 1"));
        assert!(result.contains("const B = 2"));
    }

    #[test]
    fn test_public_constants() {
        let source = r#"<?php
class Foo {
    public const X = 'x', Y = 'y';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("public const X = 'x'"));
        assert!(result.contains("public const Y = 'y'"));
    }

    #[test]
    fn test_skip_single_constant() {
        let source = r#"<?php
class Foo {
    const A = 1;
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_interface_constants() {
        let source = r#"<?php
interface Bar {
    const A = 1, B = 2;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_grouped() {
        let source = r#"<?php
class Foo {
    const A = 1, B = 2;
    const C = 3, D = 4;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }
}
