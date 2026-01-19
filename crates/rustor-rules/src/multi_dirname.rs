//! Rule: Simplify nested dirname() calls to single call with levels
//!
//! Since PHP 7.0, dirname() accepts a second argument for the number of parent directories.
//!
//! Transformations:
//! - `dirname(dirname($path))` → `dirname($path, 2)`
//! - `dirname(dirname(dirname($path)))` → `dirname($path, 3)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for nested dirname calls
pub fn check_multi_dirname<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = MultiDirnameVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct MultiDirnameVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for MultiDirnameVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_simplify_nested_dirname(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Check if expression is a dirname call and return the inner expression
fn get_dirname_inner<'a>(expr: &'a Expression<'a>, source: &str) -> Option<(&'a Expression<'a>, Option<u32>)> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        let func_name = if let Expression::Identifier(ident) = func_call.function {
            let span = ident.span();
            &source[span.start.offset as usize..span.end.offset as usize]
        } else {
            return None;
        };

        if !func_name.eq_ignore_ascii_case("dirname") {
            return None;
        }

        let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

        // Must have 1 or 2 arguments
        if args.is_empty() || args.len() > 2 {
            return None;
        }

        let inner = args[0].value();

        // Get level if specified
        let level = if args.len() == 2 {
            // Check if second argument is an integer literal
            if let Expression::Literal(Literal::Integer(int_lit)) = args[1].value() {
                let span = int_lit.span();
                let text = &source[span.start.offset as usize..span.end.offset as usize];
                text.parse::<u32>().ok()
            } else {
                // Non-literal level, can't simplify
                return None;
            }
        } else {
            Some(1)
        };

        return Some((inner, level));
    }
    None
}

/// Try to simplify nested dirname calls
fn try_simplify_nested_dirname(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check if this is a dirname call
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("dirname") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Must have 1 or 2 arguments
    if args.is_empty() || args.len() > 2 {
        return None;
    }

    // Get current level
    let current_level = if args.len() == 2 {
        if let Expression::Literal(Literal::Integer(int_lit)) = args[1].value() {
            let span = int_lit.span();
            let text = &source[span.start.offset as usize..span.end.offset as usize];
            text.parse::<u32>().ok()?
        } else {
            return None;
        }
    } else {
        1
    };

    // Check if the argument is also a dirname call
    let inner = args[0].value();
    let (innermost, inner_level) = get_dirname_inner(inner, source)?;
    let inner_level = inner_level?;

    // Continue unwrapping nested dirname calls
    let mut total_level = current_level + inner_level;
    let mut current_innermost = innermost;

    while let Some((deeper, level)) = get_dirname_inner(current_innermost, source) {
        if let Some(l) = level {
            total_level += l;
            current_innermost = deeper;
        } else {
            break;
        }
    }

    // Need at least 2 levels to be worth simplifying
    if total_level < 2 {
        return None;
    }

    // Get the innermost expression text
    let innermost_span = current_innermost.span();
    let innermost_text = &source[innermost_span.start.offset as usize..innermost_span.end.offset as usize];

    let func_span = func_call.span();
    Some(Edit::new(
        func_span,
        format!("dirname({}, {})", innermost_text, total_level),
        "Simplify nested dirname() calls",
    ))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct MultiDirnameRule;

impl Rule for MultiDirnameRule {
    fn name(&self) -> &'static str {
        "multi_dirname"
    }

    fn description(&self) -> &'static str {
        "Simplify nested dirname() calls to single call with levels"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_multi_dirname(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php70)
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
        check_multi_dirname(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_double_dirname() {
        let source = "<?php dirname(dirname($path));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php dirname($path, 2);");
    }

    #[test]
    fn test_triple_dirname() {
        let source = "<?php dirname(dirname(dirname($path)));";
        assert_eq!(transform(source), "<?php dirname($path, 3);");
    }

    #[test]
    fn test_with_existing_level() {
        let source = "<?php dirname(dirname($path, 2));";
        assert_eq!(transform(source), "<?php dirname($path, 3);");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $parent = dirname(dirname($file));";
        assert_eq!(transform(source), "<?php $parent = dirname($file, 2);");
    }

    #[test]
    fn test_in_concat() {
        let source = r#"<?php $path = dirname(dirname(__DIR__)) . '/vendor';"#;
        assert_eq!(transform(source), r#"<?php $path = dirname(__DIR__, 2) . '/vendor';"#);
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return dirname(dirname($path));";
        assert_eq!(transform(source), "<?php return dirname($path, 2);");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = dirname(dirname($x));
$b = dirname(dirname($y));
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_single_dirname() {
        let source = "<?php dirname($path);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_dirname_with_variable_level() {
        // Variable level - can't simplify
        let source = "<?php dirname(dirname($path, $level));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php basename(basename($path));";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
