//! Rule: simplify_mirror_assign (DeadCode)
//!
//! Removes useless self-assignments like `$a = $a`.
//!
//! Example transformation:
//! ```php
//! // Before
//! $result = $result;
//!
//! // After
//! // (statement removed)
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_simplify_mirror_assign<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyMirrorAssignVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyMirrorAssignVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SimplifyMirrorAssignVisitor<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn expressions_equal(&self, left: &Expression<'_>, right: &Expression<'_>) -> bool {
        // Compare by source text for simplicity
        let left_text = self.get_text(left.span());
        let right_text = self.get_text(right.span());
        left_text == right_text
    }
}

impl<'a, 's> Visitor<'a> for SimplifyMirrorAssignVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        if let Statement::Expression(expr_stmt) = stmt {
            if let Expression::Assignment(assign) = &expr_stmt.expression {
                // Check if left and right are the same
                if self.expressions_equal(&assign.lhs, &assign.rhs) {
                    // Remove the entire statement (including semicolon and newline if present)
                    let stmt_text = self.get_text(stmt.span());

                    // Create an edit that removes the statement
                    self.edits.push(Edit::new(
                        stmt.span(),
                        String::new(),
                        format!("Remove useless self-assignment: {}", stmt_text.trim()),
                    ));
                }
            }
        }
        true
    }
}

pub struct SimplifyMirrorAssignRule;

impl SimplifyMirrorAssignRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimplifyMirrorAssignRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for SimplifyMirrorAssignRule {
    fn name(&self) -> &'static str {
        "simplify_mirror_assign"
    }

    fn description(&self) -> &'static str {
        "Remove useless self-assignments like $a = $a"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_mirror_assign(program, source)
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
        check_simplify_mirror_assign(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_simple_variable() {
        let source = r#"<?php
$result = $result;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(!result.contains("$result = $result"));
    }

    #[test]
    fn test_property_self_assign() {
        let source = r#"<?php
$this->value = $this->value;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_array_access_self_assign() {
        let source = r#"<?php
$arr[0] = $arr[0];
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_different_values() {
        let source = r#"<?php
$a = $b;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_different_indices() {
        let source = r#"<?php
$arr[0] = $arr[1];
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_computation() {
        let source = r#"<?php
$a = $a + 1;
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_self_assigns() {
        let source = r#"<?php
$a = $a;
$b = $b;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_nested_property() {
        let source = r#"<?php
$obj->foo->bar = $obj->foo->bar;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
