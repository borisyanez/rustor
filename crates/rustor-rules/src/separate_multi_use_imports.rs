//! Rule: separate_multi_use_imports
//!
//! Splits multi-use import statements into separate lines.
//!
//! Pattern:
//! ```php
//! // Before
//! use Foo, Bar;
//!
//! // After
//! use Foo;
//! use Bar;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_separate_multi_use_imports<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut checker = SeparateMultiUseImportsChecker {
        source,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct SeparateMultiUseImportsChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SeparateMultiUseImportsChecker<'s> {
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
            Statement::Use(use_stmt) => {
                self.check_use_statement(use_stmt);
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

    fn check_use_statement(&mut self, use_stmt: &Use<'_>) {
        // Only handle simple sequence: use Foo, Bar;
        let items = match &use_stmt.items {
            UseItems::Sequence(seq) => &seq.items,
            // Skip typed sequences (use function/const), lists, and mixed lists
            _ => return,
        };

        // Only process if there are multiple imports
        if items.len() < 2 {
            return;
        }

        // Build separate use statements
        let mut uses = Vec::new();
        for item in items.iter() {
            let name = self.get_text(item.name.span());
            let alias = item.alias.as_ref().map(|a| {
                let alias_name = self.get_text(a.identifier.span());
                format!(" as {}", alias_name)
            }).unwrap_or_default();
            uses.push(format!("use {}{}", name, alias));
        }

        let replacement = uses.join(";\n");

        self.edits.push(Edit::new(
            use_stmt.span(),
            format!("{};", replacement),
            "Separate multi-use import into individual statements".to_string(),
        ));
    }
}

pub struct SeparateMultiUseImportsRule;

impl SeparateMultiUseImportsRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SeparateMultiUseImportsRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for SeparateMultiUseImportsRule {
    fn name(&self) -> &'static str {
        "separate_multi_use_imports"
    }

    fn description(&self) -> &'static str {
        "Split multi-use import statements into separate lines"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_separate_multi_use_imports(program, source)
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
        check_separate_multi_use_imports(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_simple_multi_use() {
        let source = r#"<?php
use Foo, Bar;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("use Foo;"));
        assert!(result.contains("use Bar;"));
    }

    #[test]
    fn test_namespaced_multi_use() {
        let source = r#"<?php
use App\Models\User, App\Models\Post;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("use App\\Models\\User;"));
        assert!(result.contains("use App\\Models\\Post;"));
    }

    #[test]
    fn test_with_alias() {
        let source = r#"<?php
use Foo as F, Bar as B;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("use Foo as F;"));
        assert!(result.contains("use Bar as B;"));
    }

    #[test]
    fn test_skip_single_use() {
        let source = r#"<?php
use Foo;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_in_namespace() {
        let source = r#"<?php
namespace App;
use Foo, Bar;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_triple_use() {
        let source = r#"<?php
use A, B, C;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("use A;"));
        assert!(result.contains("use B;"));
        assert!(result.contains("use C;"));
    }
}
