//! Rule: Remove 0 and 1 from break and continue
//!
//! Since PHP 5.4, break 0 and continue 0 are deprecated.
//! break 1 and continue 1 are equivalent to break/continue without argument.
//!
//! Transformations:
//! - `break 0;` → `break;`
//! - `break 1;` → `break;`
//! - `continue 0;` → `continue;`
//! - `continue 1;` → `continue;`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for break/continue with 0 or 1
pub fn check_remove_zero_break_continue<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveZeroBreakContinueVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveZeroBreakContinueVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RemoveZeroBreakContinueVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        match stmt {
            Statement::Break(break_stmt) => {
                if let Some(edit) = try_remove_level(&break_stmt.r#break, &break_stmt.level, &break_stmt.terminator, self.source, "break") {
                    self.edits.push(edit);
                }
            }
            Statement::Continue(continue_stmt) => {
                if let Some(edit) = try_remove_level(&continue_stmt.r#continue, &continue_stmt.level, &continue_stmt.terminator, self.source, "continue") {
                    self.edits.push(edit);
                }
            }
            _ => {}
        }
        true
    }
}

/// Try to remove level 0 or 1 from break/continue
fn try_remove_level(
    keyword: &Keyword<'_>,
    level: &Option<Expression<'_>>,
    terminator: &Terminator<'_>,
    source: &str,
    keyword_name: &str,
) -> Option<Edit> {
    let expr = level.as_ref()?;

    // Check if it's a literal integer 0 or 1
    let value = match expr {
        Expression::Literal(Literal::Integer(int_lit)) => {
            let span = int_lit.span();
            let text = &source[span.start.offset as usize..span.end.offset as usize];
            text.parse::<i64>().ok()?
        }
        _ => return None,
    };

    // Only transform 0 or 1
    if value != 0 && value != 1 {
        return None;
    }

    // Build replacement: keyword + terminator (no level)
    let keyword_span = keyword.span();
    let term_span = terminator.span();

    // Replace the entire statement with just the keyword and terminator
    let term_text = &source[term_span.start.offset as usize..term_span.end.offset as usize];

    let full_span = keyword_span.join(term_span);
    let replacement = format!("{}{}", keyword_name, term_text);

    let message = if value == 0 {
        format!("Remove {} 0", keyword_name)
    } else {
        format!("Simplify {} 1 to {}", keyword_name, keyword_name)
    };

    Some(Edit::new(full_span, replacement, message))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct RemoveZeroBreakContinueRule;

impl Rule for RemoveZeroBreakContinueRule {
    fn name(&self) -> &'static str {
        "remove_zero_break_continue"
    }

    fn description(&self) -> &'static str {
        "Remove 0 and 1 from break and continue"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_zero_break_continue(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php54)
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
        check_remove_zero_break_continue(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Break ====================

    #[test]
    fn test_break_zero() {
        let source = "<?php break 0;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php break;");
    }

    #[test]
    fn test_break_one() {
        let source = "<?php break 1;";
        assert_eq!(transform(source), "<?php break;");
    }

    // ==================== Continue ====================

    #[test]
    fn test_continue_zero() {
        let source = "<?php continue 0;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php continue;");
    }

    #[test]
    fn test_continue_one() {
        let source = "<?php continue 1;";
        assert_eq!(transform(source), "<?php continue;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_loop() {
        let source = r#"<?php
for ($i = 0; $i < 10; $i++) {
    if ($i === 5) {
        break 0;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_while() {
        let source = r#"<?php
while (true) {
    continue 0;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_switch() {
        let source = r#"<?php
switch ($x) {
    case 1:
        break 0;
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
break 0;
continue 0;
break 1;
continue 1;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 4);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_break_no_level() {
        let source = "<?php break;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_continue_no_level() {
        let source = "<?php continue;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_break_two() {
        // break 2 is valid for nested loops
        let source = "<?php break 2;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_continue_three() {
        let source = "<?php continue 3;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_variable() {
        // Variable level (requires runtime evaluation)
        let source = "<?php break $level;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
