//! Rule: Convert dirname(__FILE__) to __DIR__
//!
//! The __DIR__ constant was added in PHP 5.3 as a shorthand for dirname(__FILE__).
//!
//! Transformation:
//! - `dirname(__FILE__)` → `__DIR__`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for dirname(__FILE__) patterns
pub fn check_dirname_file_to_dir<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = DirnameFileToDirVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct DirnameFileToDirVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for DirnameFileToDirVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_convert_dirname_file(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to convert dirname(__FILE__) to __DIR__
fn try_convert_dirname_file(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name is "dirname"
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

    // Only transform if there is exactly 1 argument
    if args.len() != 1 {
        return None;
    }

    // Get the argument value
    let arg_value = args[0].value();

    // Check if the argument is __FILE__ magic constant
    if let Expression::MagicConstant(MagicConstant::File(_)) = arg_value {
        let func_call_span = func_call.span();
        return Some(Edit::new(
            func_call_span,
            "__DIR__".to_string(),
            "Convert dirname(__FILE__) to __DIR__",
        ));
    }

    None
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct DirnameFileToDirRule;

impl Rule for DirnameFileToDirRule {
    fn name(&self) -> &'static str {
        "dirname_file_to_dir"
    }

    fn description(&self) -> &'static str {
        "Convert dirname(__FILE__) to __DIR__"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_dirname_file_to_dir(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php54) // __DIR__ was added in PHP 5.3
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
        check_dirname_file_to_dir(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php dirname(__FILE__);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php __DIR__;");
    }

    #[test]
    fn test_lowercase() {
        let source = "<?php dirname(__file__);";
        assert_eq!(transform(source), "<?php __DIR__;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $dir = dirname(__FILE__);";
        assert_eq!(transform(source), "<?php $dir = __DIR__;");
    }

    #[test]
    fn test_in_concat() {
        let source = "<?php $path = dirname(__FILE__) . '/vendor';";
        assert_eq!(transform(source), "<?php $path = __DIR__ . '/vendor';");
    }

    #[test]
    fn test_in_echo() {
        let source = "<?php echo dirname(__FILE__);";
        assert_eq!(transform(source), "<?php echo __DIR__;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return dirname(__FILE__);";
        assert_eq!(transform(source), "<?php return __DIR__;");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = dirname(__FILE__);
$b = dirname(__FILE__) . '/lib';
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_dirname_dir() {
        // dirname(__DIR__) should not be transformed
        let source = "<?php dirname(__DIR__);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_dirname_variable() {
        let source = "<?php dirname($path);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_dirname_with_levels() {
        // dirname(__FILE__, 2) should not be transformed
        let source = "<?php dirname(__FILE__, 2);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php other(__FILE__);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
