//! Rule: Convert get_class($obj) to $obj::class (PHP 8.0+)
//!
//! The `$obj::class` syntax is faster and more readable than `get_class($obj)`.
//!
//! Transformations:
//! - `get_class($obj)` → `$obj::class`
//! - `get_class()` (no args, inside class) → `self::class`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for get_class() calls that can use ::class syntax
pub fn check_class_on_object<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ClassOnObjectVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ClassOnObjectVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ClassOnObjectVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(replacement) = try_transform_get_class(func_call, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Replace get_class() with ::class syntax (PHP 8.0+)",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to transform get_class(), returning the replacement if successful
fn try_transform_get_class(func_call: &FunctionCall<'_>, source: &str) -> Option<String> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !name.eq_ignore_ascii_case("get_class") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    match args.len() {
        0 => {
            // get_class() with no args → self::class
            Some("self::class".to_string())
        }
        1 => {
            // get_class($obj) → $obj::class
            let arg_span = args[0].span();
            let arg_text = &source[arg_span.start.offset as usize..arg_span.end.offset as usize];
            Some(format!("{}::class", arg_text))
        }
        _ => None, // get_class only takes 0 or 1 argument
    }
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct ClassOnObjectRule;

impl Rule for ClassOnObjectRule {
    fn name(&self) -> &'static str {
        "class_on_object"
    }

    fn description(&self) -> &'static str {
        "Convert get_class($obj) to $obj::class (PHP 8.0+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_class_on_object(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
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
        check_class_on_object(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== With Object Argument ====================

    #[test]
    fn test_get_class_with_variable() {
        let source = "<?php get_class($obj);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $obj::class;");
    }

    #[test]
    fn test_get_class_in_assignment() {
        let source = "<?php $class = get_class($object);";
        assert_eq!(transform(source), "<?php $class = $object::class;");
    }

    #[test]
    fn test_get_class_in_return() {
        let source = "<?php return get_class($this->item);";
        assert_eq!(transform(source), "<?php return $this->item::class;");
    }

    #[test]
    fn test_get_class_with_method_call() {
        let source = "<?php get_class($factory->create());";
        assert_eq!(transform(source), "<?php $factory->create()::class;");
    }

    #[test]
    fn test_get_class_with_array_access() {
        let source = "<?php get_class($objects[0]);";
        assert_eq!(transform(source), "<?php $objects[0]::class;");
    }

    // ==================== No Argument (self::class) ====================

    #[test]
    fn test_get_class_no_args() {
        let source = "<?php get_class();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php self::class;");
    }

    #[test]
    fn test_get_class_no_args_in_method() {
        let source = r#"<?php
class Foo {
    public function getName() {
        return get_class();
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_get_class_uppercase() {
        let source = "<?php GET_CLASS($obj);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_get_class_mixed_case() {
        let source = "<?php Get_Class($obj);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_get_class() {
        let source = r#"<?php
$a = get_class($obj1);
$b = get_class($obj2);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Nested Contexts ====================

    #[test]
    fn test_in_array() {
        let source = "<?php $classes = [get_class($a), get_class($b)];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_in_string_concat() {
        let source = r#"<?php echo "Class: " . get_class($obj);"#;
        assert_eq!(transform(source), r#"<?php echo "Class: " . $obj::class;"#);
    }

    #[test]
    fn test_in_comparison() {
        let source = "<?php if (get_class($obj) === 'Foo') {}";
        assert_eq!(transform(source), "<?php if ($obj::class === 'Foo') {}");
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_get_class($obj);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->get_class($other);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_static_call() {
        let source = "<?php Reflection::get_class($obj);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
