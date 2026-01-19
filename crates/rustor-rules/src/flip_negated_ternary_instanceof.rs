//! Rule: Flip negated instanceof ternary to positive form
//!
//! When a ternary has a negated instanceof in its condition, flip to positive form.
//!
//! Transformation:
//! - `!$x instanceof Y ? null : $x->method()` → `$x instanceof Y ? $x->method() : null`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for negated instanceof ternary patterns
pub fn check_flip_negated_ternary_instanceof<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = FlipNegatedTernaryInstanceofVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct FlipNegatedTernaryInstanceofVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for FlipNegatedTernaryInstanceofVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Conditional(cond) = expr {
            if let Some(replacement) = try_flip_negated_instanceof(cond, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Flip negated instanceof ternary to positive form",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to flip a negated instanceof ternary, returning the replacement if successful
fn try_flip_negated_instanceof(cond: &Conditional<'_>, source: &str) -> Option<String> {
    // Must have explicit then branch
    let then_expr = cond.then.as_ref()?;

    // Condition must be !($x instanceof Y) - UnaryPrefix with Not operator
    let negated = if let Expression::UnaryPrefix(unary) = cond.condition {
        if matches!(unary.operator, UnaryPrefixOperator::Not(_)) {
            unary.operand
        } else {
            return None;
        }
    } else {
        return None;
    };

    // The negated expression must be instanceof (which is a binary operator)
    if let Expression::Binary(binary) = negated {
        if !matches!(binary.operator, BinaryOperator::Instanceof(_)) {
            return None;
        }
    } else {
        return None;
    }

    // Get text for condition (the positive instanceof), then, and else
    let instanceof_span = negated.span();
    let instanceof_text = &source[instanceof_span.start.offset as usize..instanceof_span.end.offset as usize];

    let then_span = then_expr.span();
    let then_text = &source[then_span.start.offset as usize..then_span.end.offset as usize];

    let else_span = cond.r#else.span();
    let else_text = &source[else_span.start.offset as usize..else_span.end.offset as usize];

    // Build flipped ternary: condition ? else : then
    Some(format!("{} ? {} : {}", instanceof_text, else_text, then_text))
}

use crate::registry::{Category, Rule};

pub struct FlipNegatedTernaryInstanceofRule;

impl Rule for FlipNegatedTernaryInstanceofRule {
    fn name(&self) -> &'static str {
        "flip_negated_ternary_instanceof"
    }

    fn description(&self) -> &'static str {
        "Flip negated instanceof ternary to positive form"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_flip_negated_ternary_instanceof(program, source)
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
        check_flip_negated_ternary_instanceof(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php !$obj instanceof Product ? null : $obj->getPrice();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $obj instanceof Product ? $obj->getPrice() : null;");
    }

    #[test]
    fn test_with_class() {
        let source = "<?php !$user instanceof Admin ? 'guest' : $user->getName();";
        assert_eq!(transform(source), "<?php $user instanceof Admin ? $user->getName() : 'guest';");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $result = !$obj instanceof SomeClass ? null : $obj->method();";
        assert_eq!(transform(source), "<?php $result = $obj instanceof SomeClass ? $obj->method() : null;");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return !$item instanceof Product ? 0 : $item->getPrice();";
        assert_eq!(transform(source), "<?php return $item instanceof Product ? $item->getPrice() : 0;");
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn test_method_chain() {
        let source = "<?php !$obj instanceof Service ? null : $obj->getRepository()->find();";
        assert_eq!(transform(source), "<?php $obj instanceof Service ? $obj->getRepository()->find() : null;");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = !$x instanceof A ? null : $x->a();
$b = !$y instanceof B ? null : $y->b();
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_positive_instanceof() {
        // Positive instanceof should not be transformed
        let source = "<?php $obj instanceof Product ? $obj->getPrice() : null;";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_negation() {
        // Negation of other conditions should not be transformed
        let source = "<?php !$flag ? 'no' : 'yes';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_short_ternary() {
        // Short ternary (no then branch) should be skipped
        let source = "<?php !$obj instanceof Product ?: 'default';";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
