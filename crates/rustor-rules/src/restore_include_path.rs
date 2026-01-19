//! Rule: Convert deprecated restore_include_path() to ini_restore('include_path')
//!
//! PHP 7.4 deprecated restore_include_path(). Use ini_restore('include_path') instead.
//!
//! Transformation:
//! - `restore_include_path()` → `ini_restore('include_path')`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for deprecated restore_include_path() calls
pub fn check_restore_include_path<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RestoreIncludePathVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RestoreIncludePathVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RestoreIncludePathVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(replacement) = try_transform_restore_include_path(func_call, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Replace deprecated restore_include_path() with ini_restore() (PHP 7.4+)",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to transform restore_include_path(), returning the replacement if successful
fn try_transform_restore_include_path(func_call: &FunctionCall<'_>, source: &str) -> Option<String> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !name.eq_ignore_ascii_case("restore_include_path") {
        return None;
    }

    // restore_include_path() takes no arguments
    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
    if !args.is_empty() {
        return None;
    }

    Some("ini_restore('include_path')".to_string())
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct RestoreIncludePathRule;

impl Rule for RestoreIncludePathRule {
    fn name(&self) -> &'static str {
        "restore_include_path"
    }

    fn description(&self) -> &'static str {
        "Convert deprecated restore_include_path() to ini_restore('include_path') (PHP 7.4+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_restore_include_path(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php74)
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
        check_restore_include_path(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_basic() {
        let source = "<?php restore_include_path();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php ini_restore('include_path');");
    }

    #[test]
    fn test_in_condition() {
        let source = "<?php if ($reset) { restore_include_path(); }";
        assert_eq!(
            transform(source),
            "<?php if ($reset) { ini_restore('include_path'); }"
        );
    }

    #[test]
    fn test_uppercase() {
        let source = "<?php RESTORE_INCLUDE_PATH();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple() {
        let source = "<?php restore_include_path(); restore_include_path();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_with_args() {
        // restore_include_path doesn't take arguments
        let source = "<?php restore_include_path($path);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_restore_include_path();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->restore_include_path();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
