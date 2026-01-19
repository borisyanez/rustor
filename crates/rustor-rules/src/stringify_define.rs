//! Rule: Quote the constant name in define()
//!
//! Since PHP 7.2, the first argument of define() must be a string.
//! Before that, bare constants were allowed but deprecated.
//!
//! Transformation:
//! - `define(CONSTANT_NAME, 'value')` → `define('CONSTANT_NAME', 'value')`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for define() with bare constant name
pub fn check_stringify_define<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = StringifyDefineVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StringifyDefineVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for StringifyDefineVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_stringify_define(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to add quotes to define()'s first argument
fn try_stringify_define(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
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

    // Must have at least one argument
    if args.is_empty() {
        return None;
    }

    let first_arg = args[0].value();

    // Check if first argument is a constant access (bare constant name)
    let const_name = if let Expression::ConstantAccess(const_access) = first_arg {
        let span = const_access.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        // Already a string or other expression, skip
        return None;
    };

    // Get the span of just the first argument
    let arg_span = first_arg.span();

    // Replace bare constant with quoted string
    Some(Edit::new(
        arg_span,
        format!("'{}'", const_name),
        "Quote constant name in define()",
    ))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct StringifyDefineRule;

impl Rule for StringifyDefineRule {
    fn name(&self) -> &'static str {
        "stringify_define"
    }

    fn description(&self) -> &'static str {
        "Quote the constant name in define()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_stringify_define(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php72)
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
        check_stringify_define(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php define(MY_CONSTANT, 'value');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php define('MY_CONSTANT', 'value');");
    }

    #[test]
    fn test_with_number_value() {
        let source = "<?php define(TIMEOUT, 30);";
        assert_eq!(transform(source), "<?php define('TIMEOUT', 30);");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_function() {
        let source = r#"<?php
function setup() {
    define(APP_VERSION, '1.0');
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
define(CONST_A, 'a');
define(CONST_B, 'b');
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_already_quoted() {
        // Already has string first argument
        let source = "<?php define('MY_CONSTANT', 'value');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_double_quoted() {
        let source = r#"<?php define("MY_CONSTANT", 'value');"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_variable() {
        let source = "<?php define($name, 'value');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php defined(MY_CONSTANT);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
