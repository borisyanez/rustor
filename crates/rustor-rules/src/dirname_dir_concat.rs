//! Rule: Simplify dirname(__DIR__) . '/path' to __DIR__ . '/../path'
//!
//! When concatenating dirname(__DIR__) with a path string, convert to
//! direct __DIR__ concatenation with parent directory prefix.
//!
//! Transformation:
//! - `dirname(__DIR__) . '/vendor/autoload.php'` → `__DIR__ . '/../vendor/autoload.php'`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for dirname(__DIR__) concatenation patterns
pub fn check_dirname_dir_concat<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = DirnameDirConcatVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct DirnameDirConcatVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for DirnameDirConcatVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            if matches!(binary.operator, BinaryOperator::StringConcat(_)) {
                if let Some(edit) = try_simplify_dirname_concat(binary, self.source) {
                    self.edits.push(edit);
                    return false;
                }
            }
        }
        true
    }
}

/// Check if expression is dirname(__DIR__) with exactly 1 argument
fn is_dirname_dir(expr: &Expression<'_>, source: &str) -> bool {
    if let Expression::Call(Call::Function(func_call)) = expr {
        let func_name = if let Expression::Identifier(ident) = func_call.function {
            let span = ident.span();
            &source[span.start.offset as usize..span.end.offset as usize]
        } else {
            return false;
        };

        if !func_name.eq_ignore_ascii_case("dirname") {
            return false;
        }

        let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
        if args.len() != 1 {
            return false;
        }

        // Check if argument is __DIR__
        if let Expression::MagicConstant(MagicConstant::Directory(_)) = args[0].value() {
            return true;
        }
    }
    false
}

/// Get string value from a string literal, handling both quote types
fn get_string_value<'a>(expr: &Expression<'a>, source: &str) -> Option<(String, char)> {
    if let Expression::Literal(Literal::String(string_lit)) = expr {
        let span = string_lit.span();
        let raw = &source[span.start.offset as usize..span.end.offset as usize];

        if raw.starts_with('\'') && raw.ends_with('\'') {
            return Some((raw[1..raw.len()-1].to_string(), '\''));
        } else if raw.starts_with('"') && raw.ends_with('"') {
            return Some((raw[1..raw.len()-1].to_string(), '"'));
        }
    }
    None
}

/// Try to simplify dirname(__DIR__) . '/path' to __DIR__ . '/../path'
fn try_simplify_dirname_concat(binary: &Binary<'_>, source: &str) -> Option<Edit> {
    // Left side must be dirname(__DIR__)
    if !is_dirname_dir(binary.lhs, source) {
        return None;
    }

    // Right side must be a string literal
    let (path_value, quote) = get_string_value(binary.rhs, source)?;

    // Path must start with / (Unix) or \ (Windows)
    let new_path = if path_value.starts_with('/') {
        format!("/../{}", path_value.trim_start_matches('/'))
    } else if path_value.starts_with('\\') {
        format!("\\..\\{}", path_value.trim_start_matches('\\'))
    } else {
        return None;
    };

    let binary_span = binary.span();
    Some(Edit::new(
        binary_span,
        format!("__DIR__ . {}{}{}", quote, new_path, quote),
        "Simplify dirname(__DIR__) concatenation",
    ))
}

use crate::registry::{Category, Rule};

pub struct DirnameDirConcatRule;

impl Rule for DirnameDirConcatRule {
    fn name(&self) -> &'static str {
        "dirname_dir_concat"
    }

    fn description(&self) -> &'static str {
        "Simplify dirname(__DIR__) concatenation to __DIR__ with parent prefix"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_dirname_dir_concat(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
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
        check_dirname_dir_concat(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php dirname(__DIR__) . '/vendor/autoload.php';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php __DIR__ . '/../vendor/autoload.php';");
    }

    #[test]
    fn test_double_quotes() {
        let source = r#"<?php dirname(__DIR__) . "/lib";"#;
        assert_eq!(transform(source), r#"<?php __DIR__ . "/../lib";"#);
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $path = dirname(__DIR__) . '/config.php';";
        assert_eq!(transform(source), "<?php $path = __DIR__ . '/../config.php';");
    }

    #[test]
    fn test_in_echo() {
        let source = "<?php echo dirname(__DIR__) . '/lib';";
        assert_eq!(transform(source), "<?php echo __DIR__ . '/../lib';");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return dirname(__DIR__) . '/bootstrap.php';";
        assert_eq!(transform(source), "<?php return __DIR__ . '/../bootstrap.php';");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = dirname(__DIR__) . '/vendor/autoload.php';
$b = dirname(__DIR__) . '/config/app.php';
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_dirname_file() {
        // dirname(__FILE__) should not be transformed by this rule
        let source = "<?php dirname(__FILE__) . '/config.php';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_dirname_variable() {
        let source = "<?php dirname($dir) . '/path';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_dirname_with_levels() {
        // dirname(__DIR__, 2) should not be transformed
        let source = "<?php dirname(__DIR__, 2) . '/path';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_leading_slash() {
        // Path without leading slash should not be transformed
        let source = "<?php dirname(__DIR__) . 'path';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_variable_concat() {
        let source = "<?php dirname(__DIR__) . $path;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
