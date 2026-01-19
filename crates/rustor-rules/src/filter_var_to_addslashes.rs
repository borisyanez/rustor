//! Rule: Convert filter_var() with FILTER_SANITIZE_MAGIC_QUOTES to addslashes()
//!
//! PHP 7.4 deprecated FILTER_SANITIZE_MAGIC_QUOTES. Use addslashes() instead.
//!
//! Transformation:
//! - `filter_var($str, FILTER_SANITIZE_MAGIC_QUOTES)` → `addslashes($str)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for filter_var() with FILTER_SANITIZE_MAGIC_QUOTES
pub fn check_filter_var_to_addslashes<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = FilterVarToAddslashesVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct FilterVarToAddslashesVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for FilterVarToAddslashesVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(replacement) = try_transform_filter_var(func_call, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Replace filter_var() with FILTER_SANITIZE_MAGIC_QUOTES with addslashes() (PHP 7.4+)",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to transform filter_var with magic quotes, returning the replacement if successful
fn try_transform_filter_var(func_call: &FunctionCall<'_>, source: &str) -> Option<String> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !name.eq_ignore_ascii_case("filter_var") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Must have at least 2 arguments
    if args.len() < 2 {
        return None;
    }

    // Check if second argument is FILTER_SANITIZE_MAGIC_QUOTES constant
    let second_arg = args[1].value();
    let is_magic_quotes = match second_arg {
        Expression::Identifier(ident) => {
            let const_span = ident.span();
            let const_name = &source[const_span.start.offset as usize..const_span.end.offset as usize];
            const_name == "FILTER_SANITIZE_MAGIC_QUOTES"
        }
        Expression::ConstantAccess(access) => {
            let const_span = access.span();
            let const_text = &source[const_span.start.offset as usize..const_span.end.offset as usize];
            const_text == "FILTER_SANITIZE_MAGIC_QUOTES"
        }
        _ => false,
    };

    if !is_magic_quotes {
        return None;
    }

    // Get the first argument (the string to escape)
    let first_arg_span = args[0].span();
    let first_arg_text = &source[first_arg_span.start.offset as usize..first_arg_span.end.offset as usize];

    Some(format!("addslashes({})", first_arg_text))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct FilterVarToAddslashesRule;

impl Rule for FilterVarToAddslashesRule {
    fn name(&self) -> &'static str {
        "filter_var_to_addslashes"
    }

    fn description(&self) -> &'static str {
        "Convert filter_var($s, FILTER_SANITIZE_MAGIC_QUOTES) to addslashes($s) (PHP 7.4+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_filter_var_to_addslashes(program, source)
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
        check_filter_var_to_addslashes(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_basic() {
        let source = "<?php filter_var($str, FILTER_SANITIZE_MAGIC_QUOTES);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php addslashes($str);");
    }

    #[test]
    fn test_in_assignment() {
        let source = "<?php $escaped = filter_var($input, FILTER_SANITIZE_MAGIC_QUOTES);";
        assert_eq!(transform(source), "<?php $escaped = addslashes($input);");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return filter_var($data, FILTER_SANITIZE_MAGIC_QUOTES);";
        assert_eq!(transform(source), "<?php return addslashes($data);");
    }

    #[test]
    fn test_with_expression() {
        let source = "<?php filter_var($obj->getValue(), FILTER_SANITIZE_MAGIC_QUOTES);";
        assert_eq!(transform(source), "<?php addslashes($obj->getValue());");
    }

    #[test]
    fn test_uppercase() {
        let source = "<?php FILTER_VAR($str, FILTER_SANITIZE_MAGIC_QUOTES);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = filter_var($x, FILTER_SANITIZE_MAGIC_QUOTES);
$b = filter_var($y, FILTER_SANITIZE_MAGIC_QUOTES);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_other_filter() {
        let source = "<?php filter_var($str, FILTER_SANITIZE_STRING);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_filter_validate_email() {
        let source = "<?php filter_var($email, FILTER_VALIDATE_EMAIL);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_second_arg() {
        let source = "<?php filter_var($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_filter_var($str, FILTER_SANITIZE_MAGIC_QUOTES);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->filter_var($str, FILTER_SANITIZE_MAGIC_QUOTES);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
