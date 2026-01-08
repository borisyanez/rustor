//! Rule: switch_negated_ternary
//!
//! Swaps ternary branches when condition is negated for better readability.
//!
//! Pattern:
//! - `!$cond ? $a : $b` → `$cond ? $b : $a`
//!
//! Why: Positive conditions are easier to read than negated ones.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub struct SwitchNegatedTernaryRule;

impl Rule for SwitchNegatedTernaryRule {
    fn name(&self) -> &'static str {
        "switch_negated_ternary"
    }

    fn description(&self) -> &'static str {
        "Switch negated ternary condition for better readability"
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        let mut visitor = SwitchNegatedTernaryVisitor {
            source,
            edits: Vec::new(),
        };
        visitor.visit_program(program, source);
        visitor.edits
    }
}

struct SwitchNegatedTernaryVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> SwitchNegatedTernaryVisitor<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's> Visitor<'a> for SwitchNegatedTernaryVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Match ternary with negated condition: !$cond ? $if : $else
        if let Expression::Conditional(cond) = expr {
            // Check if condition is a boolean not: !expr
            if let Expression::UnaryPrefix(unary) = &cond.condition {
                if let UnaryPrefixOperator::Not(_) = &unary.operator {
                    // We have !$cond ? $if : $else
                    // Get the inner condition (without the !)
                    let inner_cond = self.get_text(unary.operand.span());

                    // Get the if and else branches
                    // Only handle full ternary ($cond ? $if : $else), not short ternary ($cond ?: $else)
                    if let Some(then_expr) = cond.then {
                        let if_text = self.get_text(then_expr.span());
                        let else_text = self.get_text(cond.r#else.span());

                        // Build replacement: $cond ? $else : $if (swapped)
                        let replacement = format!("{} ? {} : {}", inner_cond, else_text, if_text);

                        self.edits.push(Edit::new(
                            expr.span(),
                            replacement,
                            "Switch negated ternary for better readability",
                        ));

                        // Don't visit children - we've handled this node
                        return false;
                    }
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;

    fn parse_and_check(code: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, code);
        SwitchNegatedTernaryRule.check(&program, code)
    }

    #[test]
    fn test_simple_negated_ternary() {
        let code = r#"<?php
$result = !$upper ? $name : strtoupper($name);
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$upper ? strtoupper($name) : $name"));
    }

    #[test]
    fn test_negated_comparison() {
        let code = r#"<?php
$value = !($x > 5) ? 'small' : 'big';
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("($x > 5) ? 'big' : 'small'"));
    }

    #[test]
    fn test_skip_non_negated() {
        let code = r#"<?php
$result = $cond ? $a : $b;
"#;
        let edits = parse_and_check(code);
        assert!(edits.is_empty(), "Should not transform non-negated ternary");
    }

    #[test]
    fn test_skip_double_negation() {
        // We only handle single ! at the top level
        let code = r#"<?php
$result = !!$cond ? $a : $b;
"#;
        let edits = parse_and_check(code);
        // This will match and swap to: !$cond ? $b : $a
        // Which is technically correct - double negation with swap
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_with_function_calls() {
        let code = r#"<?php
$x = !isValid($input) ? getDefault() : process($input);
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("isValid($input) ? process($input) : getDefault()"));
    }

    #[test]
    fn test_with_method_call() {
        let code = r#"<?php
$val = !$obj->isEnabled() ? 'disabled' : 'enabled';
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$obj->isEnabled() ? 'enabled' : 'disabled'"));
    }

    #[test]
    fn test_nested_ternary_outer() {
        let code = r#"<?php
$x = !$a ? ($b ? 1 : 2) : 3;
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("$a ? 3 : ($b ? 1 : 2)"));
    }
}
