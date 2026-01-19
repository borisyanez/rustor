//! Rule: Remove case insensitive define() third argument
//!
//! Case insensitive constants are deprecated in PHP 7.3 and removed in PHP 8.0.
//! The third argument should be removed.
//!
//! Transformation:
//! - `define('FOO', 42, true)` → `define('FOO', 42)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for define() with third argument
pub fn check_sensitive_define<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SensitiveDefineVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SensitiveDefineVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SensitiveDefineVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_remove_third_arg(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to remove the third argument from define() call
fn try_remove_third_arg(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name is "define"
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("define") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Only transform if there are exactly 3 arguments
    if args.len() != 3 {
        return None;
    }

    // Build replacement with only first two arguments
    let arg1_span = args[0].span();
    let arg1 = &source[arg1_span.start.offset as usize..arg1_span.end.offset as usize];

    let arg2_span = args[1].span();
    let arg2 = &source[arg2_span.start.offset as usize..arg2_span.end.offset as usize];

    let func_call_span = func_call.span();
    Some(Edit::new(
        func_call_span,
        format!("define({}, {})", arg1, arg2),
        "Remove deprecated case insensitive constant flag",
    ))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct SensitiveDefineRule;

impl Rule for SensitiveDefineRule {
    fn name(&self) -> &'static str {
        "sensitive_define"
    }

    fn description(&self) -> &'static str {
        "Remove deprecated case insensitive constant flag from define()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_sensitive_define(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php73)
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
        check_sensitive_define(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic_true() {
        let source = "<?php define('FOO', 42, true);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php define('FOO', 42);");
    }

    #[test]
    fn test_basic_false() {
        let source = "<?php define('BAR', 'value', false);";
        assert_eq!(transform(source), "<?php define('BAR', 'value');");
    }

    #[test]
    fn test_string_value() {
        let source = "<?php define('NAME', 'John', true);";
        assert_eq!(transform(source), "<?php define('NAME', 'John');");
    }

    // ==================== Complex Values ====================

    #[test]
    fn test_array_value() {
        let source = "<?php define('ARR', [1, 2, 3], true);";
        assert_eq!(transform(source), "<?php define('ARR', [1, 2, 3]);");
    }

    #[test]
    fn test_expression_value() {
        let source = "<?php define('SUM', 1 + 2, true);";
        assert_eq!(transform(source), "<?php define('SUM', 1 + 2);");
    }

    // ==================== Case Insensitive ====================

    #[test]
    fn test_uppercase() {
        let source = "<?php DEFINE('FOO', 42, true);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
define('FOO', 1, true);
define('BAR', 2, false);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_two_args() {
        // Normal define with 2 args should be skipped
        let source = "<?php define('FOO', 42);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_one_arg() {
        let source = "<?php define('FOO');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php other_func('FOO', 42, true);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
