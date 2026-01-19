//! Rule: Convert get_called_class() to static::class
//!
//! Since PHP 5.5, static::class can be used instead of get_called_class().
//! Both provide the fully qualified class name using late static binding.
//!
//! Transformation:
//! - `get_called_class()` → `static::class`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for get_called_class() calls
pub fn check_get_called_class_to_static<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = GetCalledClassToStaticVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct GetCalledClassToStaticVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for GetCalledClassToStaticVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_convert_get_called_class(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to convert get_called_class() to static::class
fn try_convert_get_called_class(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name is "get_called_class"
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("get_called_class") {
        return None;
    }

    // Must have no arguments
    if !func_call.argument_list.arguments.is_empty() {
        return None;
    }

    // Replace the entire function call with static::class
    let func_span = func_call.span();

    Some(Edit::new(
        func_span,
        "static::class".to_string(),
        "Convert get_called_class() to static::class",
    ))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct GetCalledClassToStaticRule;

impl Rule for GetCalledClassToStaticRule {
    fn name(&self) -> &'static str {
        "get_called_class_to_static"
    }

    fn description(&self) -> &'static str {
        "Convert get_called_class() to static::class"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_get_called_class_to_static(program, source)
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
        check_get_called_class_to_static(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Pattern ====================

    #[test]
    fn test_basic() {
        let source = "<?php get_called_class();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php static::class;");
    }

    #[test]
    fn test_uppercase() {
        let source = "<?php GET_CALLED_CLASS();";
        assert_eq!(transform(source), "<?php static::class;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $class = get_called_class();";
        assert_eq!(transform(source), "<?php $class = static::class;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return get_called_class();";
        assert_eq!(transform(source), "<?php return static::class;");
    }

    #[test]
    fn test_in_var_dump() {
        let source = "<?php var_dump(get_called_class());";
        assert_eq!(transform(source), "<?php var_dump(static::class);");
    }

    #[test]
    fn test_in_class() {
        let source = r#"<?php
class SomeClass {
    public function callOnMe() {
        var_dump(get_called_class());
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_concat() {
        let source = r#"<?php echo "Class: " . get_called_class();"#;
        assert_eq!(transform(source), r#"<?php echo "Class: " . static::class;"#);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = get_called_class();
$b = get_called_class();
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_other_function() {
        let source = "<?php get_class();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_get_parent_class() {
        let source = "<?php get_parent_class();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_static_class() {
        // Already using static::class
        let source = "<?php static::class;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
