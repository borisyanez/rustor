//! Rule: Replace get_class()/get_parent_class() without arguments
//!
//! Since PHP 8.3, calling get_class() and get_parent_class() without arguments
//! is deprecated. Use self::class and parent::class instead.
//!
//! Transformations:
//! - `get_class()` → `self::class`
//! - `get_parent_class()` → `parent::class`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for get_class()/get_parent_class() without arguments
pub fn check_remove_get_class_no_args<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveGetClassNoArgsVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveGetClassNoArgsVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RemoveGetClassNoArgsVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_replace_get_class(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to replace get_class()/get_parent_class() without arguments
fn try_replace_get_class(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    let func_name_lower = func_name.to_ascii_lowercase();

    // Must have no arguments
    if !func_call.argument_list.arguments.is_empty() {
        return None;
    }

    let func_span = func_call.span();

    match func_name_lower.as_str() {
        "get_class" => Some(Edit::new(
            func_span,
            "self::class".to_string(),
            "Replace get_class() with self::class",
        )),
        "get_parent_class" => Some(Edit::new(
            func_span,
            "parent::class".to_string(),
            "Replace get_parent_class() with parent::class",
        )),
        _ => None,
    }
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct RemoveGetClassNoArgsRule;

impl Rule for RemoveGetClassNoArgsRule {
    fn name(&self) -> &'static str {
        "remove_get_class_no_args"
    }

    fn description(&self) -> &'static str {
        "Replace get_class()/get_parent_class() without arguments with self::class/parent::class"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_get_class_no_args(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php83)
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
        check_remove_get_class_no_args(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== get_class ====================

    #[test]
    fn test_get_class_no_args() {
        let source = "<?php get_class();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php self::class;");
    }

    #[test]
    fn test_get_class_in_method() {
        let source = r#"<?php
class Foo {
    public function whoAmI() {
        return get_class();
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_get_class_in_concat() {
        let source = r#"<?php echo 'Class: ' . get_class();"#;
        assert_eq!(transform(source), r#"<?php echo 'Class: ' . self::class;"#);
    }

    // ==================== get_parent_class ====================

    #[test]
    fn test_get_parent_class_no_args() {
        let source = "<?php get_parent_class();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php parent::class;");
    }

    #[test]
    fn test_get_parent_class_in_method() {
        let source = r#"<?php
class Bar extends Foo {
    public function getParent() {
        return get_parent_class();
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Combined ====================

    #[test]
    fn test_both_in_same_method() {
        let source = r#"<?php
class Example extends StdClass {
    public function whoAreYou() {
        return get_class() . ' daughter of ' . get_parent_class();
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_get_class_with_arg() {
        // get_class($obj) should NOT be transformed
        let source = "<?php get_class($this);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_get_parent_class_with_arg() {
        let source = "<?php get_parent_class($this);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php get_called_class();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
