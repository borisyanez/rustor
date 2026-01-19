//! Rule: Replace mktime() without arguments with time()
//!
//! Since PHP 7.0, mktime() without arguments is deprecated and should be replaced with time().
//!
//! Transformation:
//! - `mktime()` → `time()`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for mktime() without arguments
pub fn check_mktime_to_time<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = MktimeToTimeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct MktimeToTimeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for MktimeToTimeVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_replace_mktime(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to replace mktime() with time()
fn try_replace_mktime(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name is "mktime"
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("mktime") {
        return None;
    }

    // Only transform if there are no arguments
    if !func_call.argument_list.arguments.is_empty() {
        return None;
    }

    let func_span = func_call.span();
    Some(Edit::new(
        func_span,
        "time()".to_string(),
        "Replace mktime() with time()",
    ))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct MktimeToTimeRule;

impl Rule for MktimeToTimeRule {
    fn name(&self) -> &'static str {
        "mktime_to_time"
    }

    fn description(&self) -> &'static str {
        "Replace mktime() without arguments with time()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_mktime_to_time(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php70)
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
        check_mktime_to_time(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php mktime();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php time();");
    }

    #[test]
    fn test_uppercase() {
        let source = "<?php MKTIME();";
        assert_eq!(transform(source), "<?php time();");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $time = mktime();";
        assert_eq!(transform(source), "<?php $time = time();");
    }

    #[test]
    fn test_in_condition() {
        let source = "<?php if (mktime() > $threshold) {}";
        assert_eq!(transform(source), "<?php if (time() > $threshold) {}");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return mktime();";
        assert_eq!(transform(source), "<?php return time();");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = mktime();
$b = mktime();
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_with_args() {
        // mktime() with arguments should not be transformed
        let source = "<?php mktime(1, 2, 3);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_one_arg() {
        let source = "<?php mktime(12);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php time();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
