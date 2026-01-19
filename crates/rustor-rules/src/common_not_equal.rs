//! Rule: Use common != instead of <>
//!
//! PHP supports both <> and != for not-equal comparison, but != is more common
//! and widely recognized. Convert <> to != for consistency.
//!
//! Transformation:
//! - `$a <> $b` → `$a != $b`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for <> operator usage
pub fn check_common_not_equal<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = CommonNotEqualVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct CommonNotEqualVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for CommonNotEqualVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            if let BinaryOperator::AngledNotEqual(_) = &binary.operator {
                // Found <> operator - replace the whole expression
                let lhs_span = binary.lhs.span();
                let rhs_span = binary.rhs.span();
                let lhs = &self.source[lhs_span.start.offset as usize..lhs_span.end.offset as usize];
                let rhs = &self.source[rhs_span.start.offset as usize..rhs_span.end.offset as usize];

                self.edits.push(Edit::new(
                    expr.span(),
                    format!("{} != {}", lhs, rhs),
                    "Use common != instead of <>",
                ));
                return false;
            }
        }
        true
    }
}

use crate::registry::{Category, Rule};

pub struct CommonNotEqualRule;

impl Rule for CommonNotEqualRule {
    fn name(&self) -> &'static str {
        "common_not_equal"
    }

    fn description(&self) -> &'static str {
        "Use common != instead of <>"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_common_not_equal(program, source)
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
        check_common_not_equal(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php $a <> $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $a != $b;");
    }

    #[test]
    fn test_with_strings() {
        let source = "<?php $name <> 'admin';";
        assert_eq!(transform(source), "<?php $name != 'admin';");
    }

    #[test]
    fn test_with_numbers() {
        let source = "<?php $count <> 0;";
        assert_eq!(transform(source), "<?php $count != 0;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_condition() {
        let source = "<?php if ($x <> $y) {}";
        assert_eq!(transform(source), "<?php if ($x != $y) {}");
    }

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = $a <> $b;";
        assert_eq!(transform(source), "<?php $result = $a != $b;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return $value <> null;";
        assert_eq!(transform(source), "<?php return $value != null;");
    }

    #[test]
    fn test_in_ternary() {
        let source = "<?php $x <> $y ? 'diff' : 'same';";
        assert_eq!(transform(source), "<?php $x != $y ? 'diff' : 'same';");
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn test_with_function_calls() {
        let source = "<?php getValue() <> getOther();";
        assert_eq!(transform(source), "<?php getValue() != getOther();");
    }

    #[test]
    fn test_with_array_access() {
        let source = "<?php $arr[0] <> $arr[1];";
        assert_eq!(transform(source), "<?php $arr[0] != $arr[1];");
    }

    #[test]
    fn test_with_property() {
        let source = "<?php $obj->a <> $obj->b;";
        assert_eq!(transform(source), "<?php $obj->a != $obj->b;");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a <> $b;
$c <> $d;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_not_equal() {
        // != should not be transformed
        let source = "<?php $a != $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_identical() {
        let source = "<?php $a !== $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_equal() {
        let source = "<?php $a == $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_less_greater() {
        let source = "<?php $a < $b; $a > $b;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
