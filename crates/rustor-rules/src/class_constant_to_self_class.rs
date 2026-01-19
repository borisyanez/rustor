//! Rule: Replace __CLASS__ with self::class
//!
//! Since PHP 5.5, self::class is the preferred way to get the class name.
//!
//! Transformation:
//! - `__CLASS__` → `self::class`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for __CLASS__ magic constant
pub fn check_class_constant_to_self_class<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ClassConstantToSelfClassVisitor {
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ClassConstantToSelfClassVisitor {
    edits: Vec<Edit>,
}

impl<'a> Visitor<'a> for ClassConstantToSelfClassVisitor {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::MagicConstant(MagicConstant::Class(class_const)) = expr {
            let span = class_const.span();
            self.edits.push(Edit::new(
                span,
                "self::class".to_string(),
                "Replace __CLASS__ with self::class",
            ));
            return false;
        }
        true
    }
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct ClassConstantToSelfClassRule;

impl Rule for ClassConstantToSelfClassRule {
    fn name(&self) -> &'static str {
        "class_constant_to_self_class"
    }

    fn description(&self) -> &'static str {
        "Replace __CLASS__ with self::class"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_class_constant_to_self_class(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php55)
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
        check_class_constant_to_self_class(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php echo __CLASS__;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php echo self::class;");
    }

    #[test]
    fn test_in_class() {
        let source = r#"<?php
class Foo {
    public function getName() {
        return __CLASS__;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_var_dump() {
        let source = r#"<?php
class SomeClass {
    public function callOnMe() {
        var_dump(__CLASS__);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $class = __CLASS__;";
        assert_eq!(transform(source), "<?php $class = self::class;");
    }

    #[test]
    fn test_in_concat() {
        let source = r#"<?php echo "Class: " . __CLASS__;"#;
        assert_eq!(transform(source), r#"<?php echo "Class: " . self::class;"#);
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return __CLASS__;";
        assert_eq!(transform(source), "<?php return self::class;");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = __CLASS__;
$b = __CLASS__;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_other_magic_constants() {
        // Other magic constants should not be transformed
        let source = "<?php echo __FILE__;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_dir() {
        let source = "<?php echo __DIR__;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_function() {
        let source = "<?php echo __FUNCTION__;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method() {
        let source = "<?php echo __METHOD__;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_self_class() {
        // Already using self::class
        let source = "<?php echo self::class;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
