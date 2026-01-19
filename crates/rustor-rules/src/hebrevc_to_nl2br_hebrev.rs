//! Rule: Convert deprecated hebrevc() to nl2br(hebrev())
//!
//! PHP 7.4 deprecated hebrevc(). Use nl2br(hebrev()) instead.
//! hebrevc() converts Hebrew text and adds <br> for newlines.
//!
//! Transformation:
//! - `hebrevc($str)` → `nl2br(hebrev($str))`
//! - `hebrevc($str, $max)` → `nl2br(hebrev($str, $max))`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for deprecated hebrevc() calls
pub fn check_hebrevc_to_nl2br_hebrev<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = HebrevcToNl2brHebrevVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct HebrevcToNl2brHebrevVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for HebrevcToNl2brHebrevVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(replacement) = try_transform_hebrevc(func_call, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Replace deprecated hebrevc() with nl2br(hebrev()) (PHP 7.4+)",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to transform hebrevc(), returning the replacement if successful
fn try_transform_hebrevc(func_call: &FunctionCall<'_>, source: &str) -> Option<String> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !name.eq_ignore_ascii_case("hebrevc") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // hebrevc takes 1 or 2 arguments
    if args.is_empty() || args.len() > 2 {
        return None;
    }

    // Get all arguments text
    let first_span = args.first().unwrap().span();
    let last_span = args.last().unwrap().span();
    let args_text = &source[first_span.start.offset as usize..last_span.end.offset as usize];

    // hebrevc($str) → nl2br(hebrev($str))
    // hebrevc($str, $max) → nl2br(hebrev($str, $max))
    Some(format!("nl2br(hebrev({}))", args_text))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct HebrevcToNl2brHebrevRule;

impl Rule for HebrevcToNl2brHebrevRule {
    fn name(&self) -> &'static str {
        "hebrevc_to_nl2br_hebrev"
    }

    fn description(&self) -> &'static str {
        "Convert deprecated hebrevc() to nl2br(hebrev()) (PHP 7.4+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_hebrevc_to_nl2br_hebrev(program, source)
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
        check_hebrevc_to_nl2br_hebrev(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_basic() {
        let source = "<?php hebrevc($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php nl2br(hebrev($str));");
    }

    #[test]
    fn test_with_max_chars() {
        let source = "<?php hebrevc($str, 50);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php nl2br(hebrev($str, 50));");
    }

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = hebrevc($text);";
        assert_eq!(transform(source), "<?php $result = nl2br(hebrev($text));");
    }

    #[test]
    fn test_in_echo() {
        let source = "<?php echo hebrevc($hebrew);";
        assert_eq!(transform(source), "<?php echo nl2br(hebrev($hebrew));");
    }

    #[test]
    fn test_with_expression() {
        let source = "<?php hebrevc($obj->getText());";
        assert_eq!(transform(source), "<?php nl2br(hebrev($obj->getText()));");
    }

    #[test]
    fn test_uppercase() {
        let source = "<?php HEBREVC($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple() {
        let source = "<?php hebrevc($a); hebrevc($b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_no_args() {
        let source = "<?php hebrevc();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_too_many_args() {
        let source = "<?php hebrevc($str, 50, $extra);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_hebrevc($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->hebrevc($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_hebrev() {
        // Don't transform hebrev itself
        let source = "<?php hebrev($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
