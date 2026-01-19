//! Rule: Replace double boolean not (!!) with (bool) cast
//!
//! The double negation pattern !!$x is commonly used to convert a value to boolean,
//! but (bool) $x is more readable and explicit.
//!
//! Transformations:
//! - `!!$x` → `(bool) $x`
//! - `!!!$x` → `!$x` (triple negation simplifies to single)
//! - `!!!!$x` → `(bool) $x` (quad negation simplifies to cast)

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for double negation patterns
pub fn check_double_negation_to_cast<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = DoubleNegationVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct DoubleNegationVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for DoubleNegationVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::UnaryPrefix(unary) = expr {
            if let Some(replacement) = try_transform_double_negation(unary, self.source) {
                // Use the full expression span for the edit
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Replace double negation with (bool) cast",
                ));
                // Don't visit children - we've handled this expression
                return false;
            }
        }
        true
    }
}

/// Try to transform double negation, returning the replacement if successful
fn try_transform_double_negation<'a>(
    unary: &UnaryPrefix<'a>,
    source: &str,
) -> Option<String> {
    // Must start with a Not operator
    if !matches!(unary.operator, UnaryPrefixOperator::Not(_)) {
        return None;
    }

    // Count consecutive negations starting from this unary
    let mut count = 1;
    let mut current = unary.operand;

    while let Expression::UnaryPrefix(inner_unary) = current {
        if matches!(inner_unary.operator, UnaryPrefixOperator::Not(_)) {
            count += 1;
            current = inner_unary.operand;
        } else {
            break;
        }
    }

    // Need at least 2 negations
    if count < 2 {
        return None;
    }

    // Get the innermost expression text
    let inner_span = current.span();
    let inner_text = &source[inner_span.start.offset as usize..inner_span.end.offset as usize];

    // Determine replacement based on whether we have odd or even negations
    let replacement = if count % 2 == 0 {
        // Even number of negations -> (bool) cast
        format!("(bool) {}", inner_text)
    } else {
        // Odd number of negations -> single negation
        format!("!{}", inner_text)
    };

    Some(replacement)
}

use crate::registry::{Category, Rule};

pub struct DoubleNegationToCastRule;

impl Rule for DoubleNegationToCastRule {
    fn name(&self) -> &'static str {
        "double_negation_to_cast"
    }

    fn description(&self) -> &'static str {
        "Replace double negation (!!) with (bool) cast"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_double_negation_to_cast(program, source)
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
        check_double_negation_to_cast(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_double_negation() {
        let source = "<?php !!$var;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php (bool) $var;");
    }

    #[test]
    fn test_triple_negation() {
        let source = "<?php !!!$var;";
        assert_eq!(transform(source), "<?php !$var;");
    }

    #[test]
    fn test_quad_negation() {
        let source = "<?php !!!!$var;";
        assert_eq!(transform(source), "<?php (bool) $var;");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $bool = !!$value;";
        assert_eq!(transform(source), "<?php $bool = (bool) $value;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return !!$var;";
        assert_eq!(transform(source), "<?php return (bool) $var;");
    }

    #[test]
    fn test_in_condition() {
        let source = "<?php if (!!$x) {}";
        assert_eq!(transform(source), "<?php if ((bool) $x) {}");
    }

    #[test]
    fn test_in_array() {
        let source = "<?php $arr = [!!$a, !!$b];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn test_with_expression() {
        let source = "<?php !!($a && $b);";
        assert_eq!(transform(source), "<?php (bool) ($a && $b);");
    }

    #[test]
    fn test_with_function_call() {
        let source = "<?php !!getValue();";
        assert_eq!(transform(source), "<?php (bool) getValue();");
    }

    #[test]
    fn test_with_property() {
        let source = "<?php !!$obj->prop;";
        assert_eq!(transform(source), "<?php (bool) $obj->prop;");
    }

    #[test]
    fn test_with_array_access() {
        let source = "<?php !!$arr[0];";
        assert_eq!(transform(source), "<?php (bool) $arr[0];");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = !!$x;
$b = !!$y;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_single_negation() {
        let source = "<?php !$var;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_negation() {
        let source = "<?php $var;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_unary() {
        // Bitwise not is different
        let source = "<?php ~~$var;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
