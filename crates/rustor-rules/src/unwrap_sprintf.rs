//! Rule: Unwrap sprintf() with one argument
//!
//! When sprintf() is called with only one argument (no placeholders),
//! it can be simplified to just the argument itself.
//!
//! Transformation:
//! - `sprintf('value')` → `'value'`
//! - `sprintf($var)` → `$var`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for sprintf() calls with one argument
pub fn check_unwrap_sprintf<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = UnwrapSprintfVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct UnwrapSprintfVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for UnwrapSprintfVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(replacement) = try_transform_sprintf(func_call, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Unwrap sprintf() with single argument",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to transform sprintf() with one arg, returning the replacement if successful
fn try_transform_sprintf(func_call: &FunctionCall<'_>, source: &str) -> Option<String> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !name.eq_ignore_ascii_case("sprintf") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Must have exactly 1 argument
    if args.len() != 1 {
        return None;
    }

    let arg = &args[0];

    // Get the argument value
    let arg_span = arg.span();
    let arg_text = &source[arg_span.start.offset as usize..arg_span.end.offset as usize];

    Some(arg_text.to_string())
}

use crate::registry::{Category, Rule};

pub struct UnwrapSprintfRule;

impl Rule for UnwrapSprintfRule {
    fn name(&self) -> &'static str {
        "unwrap_sprintf"
    }

    fn description(&self) -> &'static str {
        "Unwrap sprintf() with single argument"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_unwrap_sprintf(program, source)
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
        check_unwrap_sprintf(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_string_literal() {
        let source = "<?php sprintf('value');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php 'value';");
    }

    #[test]
    fn test_variable() {
        let source = "<?php sprintf($var);";
        assert_eq!(transform(source), "<?php $var;");
    }

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = sprintf('hello');";
        assert_eq!(transform(source), "<?php $result = 'hello';");
    }

    #[test]
    fn test_in_echo() {
        let source = "<?php echo sprintf('message');";
        assert_eq!(transform(source), "<?php echo 'message';");
    }

    #[test]
    fn test_with_expression() {
        let source = "<?php sprintf($obj->getMessage());";
        assert_eq!(transform(source), "<?php $obj->getMessage();");
    }

    #[test]
    fn test_double_quoted() {
        let source = r#"<?php sprintf("value");"#;
        assert_eq!(transform(source), r#"<?php "value";"#);
    }

    #[test]
    fn test_uppercase() {
        let source = "<?php SPRINTF('test');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple() {
        let source = "<?php sprintf('a'); sprintf('b');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_with_placeholder() {
        let source = "<?php sprintf('Hello %s', $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_multiple_args() {
        let source = "<?php sprintf('%s %s', $a, $b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_args() {
        let source = "<?php sprintf();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->sprintf('value');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_sprintf('value');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
