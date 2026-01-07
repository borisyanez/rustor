//! Rule: Convert null checks to nullsafe operator (PHP 8.0+)
//!
//! Example:
//! ```php
//! // Before
//! $value = $user ? $user->getProfile() : null;
//! $name = $user !== null ? $user->getName() : null;
//! $data = $obj != null ? $obj->data : null;
//!
//! // After
//! $value = $user?->getProfile();
//! $name = $user?->getName();
//! $data = $obj?->data;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for null check patterns that can use nullsafe operator
pub fn check_null_safe_operator<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = NullSafeOperatorVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct NullSafeOperatorVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for NullSafeOperatorVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Conditional(cond) = expr {
            self.check_conditional(cond);
        }
        true // Continue traversal
    }
}

impl<'s> NullSafeOperatorVisitor<'s> {
    fn check_conditional(&mut self, cond: &Conditional<'_>) {
        // Get the "then" expression - it must exist for this pattern
        let then_expr = match cond.then {
            Some(expr) => expr,
            None => return, // Short ternary (?:) can't be converted
        };

        // Check if "else" is null
        if !self.is_null_literal(cond.r#else) {
            return;
        }

        // Check if the condition is a null check pattern
        let checked_var = match self.extract_null_check_var(&cond.condition) {
            Some(v) => v,
            None => return,
        };

        // Check if "then" is a method call or property access on the same variable
        let (base_var, access_chain) = match self.extract_access_chain(then_expr) {
            Some(v) => v,
            None => return,
        };

        // Verify the base variable matches the checked variable
        if !self.vars_equal(&checked_var, &base_var) {
            return;
        }

        // Create the nullsafe version
        let span = cond.span();
        let replacement = format!("{}{}", base_var, access_chain);

        self.edits.push(Edit::new(
            span,
            replacement,
            "Convert null check to nullsafe operator (PHP 8.0+)",
        ));
    }

    /// Check if an expression is a null literal
    fn is_null_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::Null(_)))
    }

    /// Extract the variable being checked for null from a condition
    /// Returns the variable code if pattern matches:
    /// - $var (truthy check)
    /// - $var !== null
    /// - $var != null
    /// - null !== $var
    /// - null != $var
    fn extract_null_check_var(&self, condition: &Expression<'_>) -> Option<String> {
        match condition {
            // Simple truthy check: $var ? ... : null
            Expression::Variable(var) => {
                let span = var.span();
                Some(self.source[span.start.offset as usize..span.end.offset as usize].to_string())
            }
            // Comparison check: $var !== null or null !== $var
            Expression::Binary(binary) => {
                // Must be !== or != (not identical/equal)
                let is_not_null_check = matches!(
                    &binary.operator,
                    BinaryOperator::NotIdentical(_) | BinaryOperator::NotEqual(_)
                );

                if !is_not_null_check {
                    return None;
                }

                // Check $var !== null
                if self.is_null_literal(binary.rhs) {
                    if let Expression::Variable(var) = binary.lhs {
                        let span = var.span();
                        return Some(self.source[span.start.offset as usize..span.end.offset as usize].to_string());
                    }
                }

                // Check null !== $var
                if self.is_null_literal(binary.lhs) {
                    if let Expression::Variable(var) = binary.rhs {
                        let span = var.span();
                        return Some(self.source[span.start.offset as usize..span.end.offset as usize].to_string());
                    }
                }

                None
            }
            // Parenthesized expression
            Expression::Parenthesized(paren) => {
                self.extract_null_check_var(paren.expression)
            }
            _ => None,
        }
    }

    /// Extract the base variable and access chain from a property access or method call
    /// Returns (base_var, access_chain) where access_chain uses ?-> instead of ->
    fn extract_access_chain(&self, expr: &Expression<'_>) -> Option<(String, String)> {
        match expr {
            // Property access: $var->prop
            Expression::Access(Access::Property(prop)) => {
                let base_var = self.extract_var_from_expr(prop.object)?;
                let member_span = prop.property.span();
                let member = &self.source[member_span.start.offset as usize..member_span.end.offset as usize];
                Some((base_var, format!("?->{}", member)))
            }
            // Method call: $var->method()
            Expression::Call(Call::Method(method)) => {
                let base_var = self.extract_var_from_expr(method.object)?;

                // Get the method name
                let method_span = method.method.span();
                let method_name = &self.source[method_span.start.offset as usize..method_span.end.offset as usize];

                // Get the arguments
                let args_span = method.argument_list.span();
                let args = &self.source[args_span.start.offset as usize..args_span.end.offset as usize];

                Some((base_var, format!("?->{}{}", method_name, args)))
            }
            // Nullsafe property access (already using ?->)
            Expression::Access(Access::NullSafeProperty(_)) => None,
            // Nullsafe method call (already using ?->)
            Expression::Call(Call::NullSafeMethod(_)) => None,
            _ => None,
        }
    }

    /// Extract a simple variable from an expression
    fn extract_var_from_expr(&self, expr: &Expression<'_>) -> Option<String> {
        if let Expression::Variable(var) = expr {
            let span = var.span();
            return Some(self.source[span.start.offset as usize..span.end.offset as usize].to_string());
        }
        None
    }

    /// Check if two variable strings are equal
    fn vars_equal(&self, a: &str, b: &str) -> bool {
        a == b
    }
}

pub struct NullSafeOperatorRule;

impl Rule for NullSafeOperatorRule {
    fn name(&self) -> &'static str {
        "null_safe_operator"
    }

    fn description(&self) -> &'static str {
        "Convert null checks to nullsafe operator ?->"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_null_safe_operator(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
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
        check_null_safe_operator(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_truthy_check_method_call() {
        let source = r#"<?php
$value = $user ? $user->getProfile() : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$user?->getProfile()"));
    }

    #[test]
    fn test_truthy_check_property_access() {
        let source = r#"<?php
$name = $user ? $user->name : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$user?->name"));
    }

    #[test]
    fn test_not_identical_null_check() {
        let source = r#"<?php
$value = $user !== null ? $user->getName() : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$user?->getName()"));
    }

    #[test]
    fn test_not_equal_null_check() {
        let source = r#"<?php
$value = $obj != null ? $obj->data : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$obj?->data"));
    }

    #[test]
    fn test_reversed_null_check() {
        let source = r#"<?php
$value = null !== $user ? $user->getName() : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$user?->getName()"));
    }

    #[test]
    fn test_method_with_arguments() {
        let source = r#"<?php
$value = $service ? $service->process($data, true) : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$service?->process($data, true)"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_different_else_value() {
        // Else is not null
        let source = r#"<?php
$value = $user ? $user->getName() : 'default';
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_different_variable() {
        // Different variable in condition and access
        let source = r#"<?php
$value = $user ? $other->getName() : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_short_ternary() {
        // Short ternary (?:) can't use nullsafe
        let source = r#"<?php
$value = $user ?: null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_null_comparison() {
        // Comparison with something other than null
        let source = r#"<?php
$value = $user !== false ? $user->getName() : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_identical_null() {
        // === null is the opposite pattern
        let source = r#"<?php
$value = $user === null ? $user->getName() : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_static_method() {
        // Static method calls don't use nullsafe
        let source = r#"<?php
$value = $class ? $class::getStatic() : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_patterns() {
        let source = r#"<?php
$a = $user ? $user->name : null;
$b = $obj !== null ? $obj->getValue() : null;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        let result = transform(source);
        assert!(result.contains("$user?->name"));
        assert!(result.contains("$obj?->getValue()"));
    }
}
