//! Rule: split_grouped_properties
//!
//! Splits grouped property declarations into separate statements.
//!
//! Pattern:
//! ```php
//! // Before
//! class Foo {
//!     public $a, $b;
//! }
//!
//! // After
//! class Foo {
//!     public $a;
//!     public $b;
//! }
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_split_grouped_properties<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut checker = SplitGroupedPropertiesChecker {
        source,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct SplitGroupedPropertiesChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SplitGroupedPropertiesChecker<'s> {
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
            Statement::Trait(trait_def) => {
                self.check_trait(trait_def);
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
            if let ClassLikeMember::Property(Property::Plain(prop)) = member {
                self.check_plain_property(prop);
            }
        }
    }

    fn check_trait(&mut self, trait_def: &Trait<'_>) {
        for member in trait_def.members.iter() {
            if let ClassLikeMember::Property(Property::Plain(prop)) = member {
                self.check_plain_property(prop);
            }
        }
    }

    fn check_plain_property(&mut self, prop: &PlainProperty<'_>) {
        // Only process if there are multiple properties in one declaration
        if prop.items.len() < 2 {
            return;
        }

        // Build modifiers string
        let mut modifiers = String::new();
        for modifier in prop.modifiers.iter() {
            modifiers.push_str(self.get_text(modifier.span()));
            modifiers.push(' ');
        }

        // Build type hint if present
        let type_hint = prop
            .hint
            .as_ref()
            .map(|h| format!("{} ", self.get_text(h.span())))
            .unwrap_or_default();

        // Build separate property declarations
        let mut declarations = Vec::new();
        for item in prop.items.iter() {
            let var_text = self.get_text(item.variable().span());
            let value_text = match item {
                PropertyItem::Concrete(concrete) => {
                    format!(" = {}", self.get_text(concrete.value.span()))
                }
                PropertyItem::Abstract(_) => String::new(),
            };
            declarations.push(format!("{}{}{}{}", modifiers, type_hint, var_text, value_text));
        }

        let replacement = declarations.join(";\n    ");

        self.edits.push(Edit::new(
            prop.span(),
            format!("{};", replacement),
            "Split grouped properties into separate declarations".to_string(),
        ));
    }
}

pub struct SplitGroupedPropertiesRule;

impl SplitGroupedPropertiesRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SplitGroupedPropertiesRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for SplitGroupedPropertiesRule {
    fn name(&self) -> &'static str {
        "split_grouped_properties"
    }

    fn description(&self) -> &'static str {
        "Split grouped property declarations into separate statements"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_split_grouped_properties(program, source)
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
        check_split_grouped_properties(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_simple_grouped_properties() {
        let source = r#"<?php
class Foo {
    public $a, $b;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("public $a"));
        assert!(result.contains("public $b"));
    }

    #[test]
    fn test_private_properties() {
        let source = r#"<?php
class Foo {
    private $x, $y;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("private $x"));
        assert!(result.contains("private $y"));
    }

    #[test]
    fn test_with_defaults() {
        let source = r#"<?php
class Foo {
    public $a = 1, $b = 2;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("public $a = 1"));
        assert!(result.contains("public $b = 2"));
    }

    #[test]
    fn test_skip_single_property() {
        let source = r#"<?php
class Foo {
    public $a;
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_trait_properties() {
        let source = r#"<?php
trait Bar {
    protected $a, $b;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_typed_properties() {
        let source = r#"<?php
class Foo {
    public int $a, $b;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("public int $a"));
        assert!(result.contains("public int $b"));
    }
}
