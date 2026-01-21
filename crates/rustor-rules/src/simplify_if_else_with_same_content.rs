//! Rule: simplify_if_else_with_same_content (DeadCode)
//!
//! Simplifies if/else statements where both branches have the same content.
//!
//! Example transformation:
//! ```php
//! // Before
//! if ($condition) {
//!     doSomething();
//! } else {
//!     doSomething();
//! }
//!
//! // After
//! doSomething();
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_simplify_if_else_with_same_content<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyIfElseVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyIfElseVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SimplifyIfElseVisitor<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Get the body content text from an if statement body, normalized
    fn get_body_content(&self, body: &IfBody<'_>) -> Option<String> {
        match body {
            IfBody::Statement(stmt_body) => {
                // Get the statement text
                let stmt_text = self.get_text(stmt_body.statement.span()).trim();
                Some(self.normalize_body_content(stmt_text))
            }
            IfBody::ColonDelimited(block) => {
                // Get text of all statements in block, normalized
                let stmts: Vec<&str> = block.statements.iter()
                    .map(|s: &Statement<'_>| self.get_text(s.span()).trim())
                    .collect();
                Some(self.normalize_body_content(&stmts.join("\n")))
            }
        }
    }

    /// Get else clause body content
    fn get_else_content(&self, body: &IfBody<'_>) -> Option<String> {
        match body {
            IfBody::Statement(stmt_body) => {
                if let Some(else_clause) = &stmt_body.else_clause {
                    let stmt_text = self.get_text(else_clause.statement.span()).trim();
                    Some(self.normalize_body_content(stmt_text))
                } else {
                    None
                }
            }
            IfBody::ColonDelimited(block) => {
                if let Some(else_clause) = &block.else_clause {
                    let stmts: Vec<&str> = else_clause.statements.iter()
                        .map(|s: &Statement<'_>| self.get_text(s.span()).trim())
                        .collect();
                    Some(self.normalize_body_content(&stmts.join("\n")))
                } else {
                    None
                }
            }
        }
    }

    /// Get the body text to use as replacement
    fn get_replacement_body(&self, body: &IfBody<'_>) -> String {
        match body {
            IfBody::Statement(stmt_body) => {
                self.get_text(stmt_body.statement.span()).to_string()
            }
            IfBody::ColonDelimited(block) => {
                block.statements.iter()
                    .map(|s: &Statement<'_>| self.get_text(s.span()))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }

    /// Normalize body content for comparison (remove extra whitespace)
    fn normalize_body_content(&self, content: &str) -> String {
        content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if there are elseif clauses
    fn has_elseif_clauses(&self, body: &IfBody<'_>) -> bool {
        match body {
            IfBody::Statement(stmt_body) => !stmt_body.else_if_clauses.is_empty(),
            IfBody::ColonDelimited(block) => !block.else_if_clauses.is_empty(),
        }
    }
}

impl<'a, 's> Visitor<'a> for SimplifyIfElseVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        if let Statement::If(if_stmt) = stmt {
            // Skip if there are elseif clauses - too complex
            if self.has_elseif_clauses(&if_stmt.body) {
                return true;
            }

            // Get if body content
            if let Some(if_content) = self.get_body_content(&if_stmt.body) {
                // Get else body content
                if let Some(else_content) = self.get_else_content(&if_stmt.body) {
                    // Compare normalized content
                    if if_content == else_content {
                        let replacement = self.get_replacement_body(&if_stmt.body);
                        self.edits.push(Edit::new(
                            stmt.span(),
                            replacement,
                            "Remove if/else with same content".to_string(),
                        ));
                        return true;
                    }
                }
            }
        }
        true
    }
}

pub struct SimplifyIfElseWithSameContentRule;

impl SimplifyIfElseWithSameContentRule {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimplifyIfElseWithSameContentRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for SimplifyIfElseWithSameContentRule {
    fn name(&self) -> &'static str {
        "simplify_if_else_with_same_content"
    }

    fn description(&self) -> &'static str {
        "Remove if/else where both branches have the same content"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_if_else_with_same_content(program, source)
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
        check_simplify_if_else_with_same_content(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_simple_same_content() {
        let source = r#"<?php
if ($condition) {
    doSomething();
} else {
    doSomething();
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("doSomething();"));
        assert!(!result.contains("if"));
        assert!(!result.contains("else"));
    }

    #[test]
    fn test_multiline_same_content() {
        let source = r#"<?php
if ($x) {
    $a = 1;
    $b = 2;
} else {
    $a = 1;
    $b = 2;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_different_content() {
        let source = r#"<?php
if ($condition) {
    doSomething();
} else {
    doOther();
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_no_else() {
        let source = r#"<?php
if ($condition) {
    doSomething();
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_elseif() {
        let source = r#"<?php
if ($a) {
    doSomething();
} elseif ($b) {
    doOther();
} else {
    doSomething();
}
"#;
        let edits = check_php(source);
        // We skip elseif cases as they're complex
        assert!(edits.is_empty());
    }

    #[test]
    fn test_inline_if_else() {
        let source = r#"<?php
if ($x) echo "yes"; else echo "yes";
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_whitespace_differences() {
        let source = r#"<?php
if ($x) {
    doSomething();
} else {
    doSomething()  ;
}
"#;
        // Different whitespace but same logical content - may or may not match
        // depending on normalization. Let's test what happens.
        let edits = check_php(source);
        // With our normalization, "doSomething();" vs "doSomething()  ;" are different
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_same_ifs() {
        let source = r#"<?php
if ($a) {
    foo();
} else {
    foo();
}
if ($b) {
    bar();
} else {
    bar();
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }
}
