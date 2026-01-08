//! Rule: Convert simple closures to arrow functions (PHP 7.4+)
//!
//! Example:
//! ```php
//! // Before
//! $double = function($x) { return $x * 2; };
//! array_map(function($x) { return $x * 2; }, $arr);
//!
//! // After
//! $double = fn($x) => $x * 2;
//! array_map(fn($x) => $x * 2, $arr);
//! ```
//!
//! Requirements for conversion:
//! - Closure must have exactly one statement in the body
//! - That statement must be a return statement with a value
//! - Closure must not use references (&$var) in use() clause
//! - Closure must not have a return type hint (arrow functions infer it)

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for closures that can be converted to arrow functions
pub fn check_arrow_functions<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ArrowFunctionVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ArrowFunctionVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ArrowFunctionVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Closure(closure) = expr {
            if let Some(edit) = try_convert_closure(closure, self.source) {
                self.edits.push(edit);
            }
        }
        true // Continue traversal
    }
}

/// Try to convert a closure to an arrow function
fn try_convert_closure(closure: &Closure<'_>, source: &str) -> Option<Edit> {
    // Skip if closure has a return type hint (arrow functions don't support explicit return types)
    if closure.return_type_hint.is_some() {
        return None;
    }

    // Skip if closure uses references in use() clause
    if let Some(use_clause) = &closure.use_clause {
        for var in use_clause.variables.nodes.iter() {
            if var.ampersand.is_some() {
                return None;
            }
        }
    }

    // Closure body must have exactly one statement
    let statements: Vec<_> = closure.body.statements.iter().collect();
    if statements.len() != 1 {
        return None;
    }

    // That statement must be a return with a value
    let return_stmt = match statements[0] {
        Statement::Return(ret) => ret,
        _ => return None,
    };

    // Must have a return value
    let return_value = match &return_stmt.value {
        Some(expr) => expr,
        None => return None,
    };

    // Build the arrow function
    let mut result = String::new();

    // Add attributes if present (iterate through each attribute list)
    for attr_list in closure.attribute_lists.iter() {
        let attr_span = attr_list.span();
        let attrs = &source[attr_span.start.offset as usize..attr_span.end.offset as usize];
        result.push_str(attrs);
        result.push(' ');
    }

    // Add static if present
    if closure.r#static.is_some() {
        result.push_str("static ");
    }

    // fn keyword and parameters
    result.push_str("fn");

    // Parameters
    let params_span = closure.parameter_list.span();
    let params = &source[params_span.start.offset as usize..params_span.end.offset as usize];
    result.push_str(params);

    // Use clause - arrow functions auto-capture by value
    // If the closure has a use clause without references, we can still convert
    // (arrow functions implicitly capture all variables from parent scope)

    // Arrow and return expression
    result.push_str(" => ");
    let value_span = return_value.span();
    let value = &source[value_span.start.offset as usize..value_span.end.offset as usize];
    result.push_str(value);

    Some(Edit::new(
        closure.span(),
        result,
        "Convert closure to arrow function (PHP 7.4+)",
    ))
}

pub struct ArrowFunctionsRule;

impl Rule for ArrowFunctionsRule {
    fn name(&self) -> &'static str {
        "arrow_functions"
    }

    fn description(&self) -> &'static str {
        "Convert simple closures to arrow functions"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_arrow_functions(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
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
        check_arrow_functions(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Tests ====================

    #[test]
    fn test_rule_exists() {
        let rule = ArrowFunctionsRule;
        assert_eq!(rule.name(), "arrow_functions");
        assert_eq!(rule.min_php_version(), Some(PhpVersion::Php74));
    }

    #[test]
    fn test_simple_closure() {
        let source = r#"<?php $double = function($x) { return $x * 2; };"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert_eq!(result, r#"<?php $double = fn($x) => $x * 2;"#);
    }

    #[test]
    fn test_closure_in_array_map() {
        let source = r#"<?php array_map(function($x) { return $x * 2; }, $arr);"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert_eq!(result, r#"<?php array_map(fn($x) => $x * 2, $arr);"#);
    }

    #[test]
    fn test_closure_with_multiple_params() {
        let source = r#"<?php $add = function($a, $b) { return $a + $b; };"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert_eq!(result, r#"<?php $add = fn($a, $b) => $a + $b;"#);
    }

    #[test]
    fn test_closure_with_typed_params() {
        let source = r#"<?php $fn = function(int $x): int { return $x * 2; };"#;
        let edits = check_php(source);
        // Should skip because of return type hint
        assert!(edits.is_empty());
    }

    #[test]
    fn test_closure_with_use_by_value() {
        let source = r#"<?php $fn = function($x) use ($y) { return $x + $y; };"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        // Arrow functions auto-capture, so use clause is dropped
        assert_eq!(result, r#"<?php $fn = fn($x) => $x + $y;"#);
    }

    #[test]
    fn test_closure_with_string_return() {
        let source = r#"<?php $greet = function($name) { return "Hello, $name"; };"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert_eq!(result, r#"<?php $greet = fn($name) => "Hello, $name";"#);
    }

    #[test]
    fn test_static_closure() {
        let source = r#"<?php $fn = static function($x) { return $x * 2; };"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert_eq!(result, r#"<?php $fn = static fn($x) => $x * 2;"#);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_use_by_reference() {
        let source = r#"<?php $fn = function($x) use (&$y) { return $x + $y; };"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip closures with use by reference");
    }

    #[test]
    fn test_skip_multiple_statements() {
        let source = r#"<?php $fn = function($x) { $y = $x * 2; return $y; };"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip closures with multiple statements");
    }

    #[test]
    fn test_skip_no_return() {
        let source = r#"<?php $fn = function($x) { echo $x; };"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip closures without return");
    }

    #[test]
    fn test_skip_empty_return() {
        let source = r#"<?php $fn = function() { return; };"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip closures with empty return");
    }

    #[test]
    fn test_skip_return_type_hint() {
        let source = r#"<?php $fn = function($x): int { return $x * 2; };"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip closures with return type hint");
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn test_closure_returning_array() {
        let source = r#"<?php $fn = function($x) { return [$x, $x * 2]; };"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert_eq!(result, r#"<?php $fn = fn($x) => [$x, $x * 2];"#);
    }

    #[test]
    fn test_closure_returning_ternary() {
        let source = r#"<?php $fn = function($x) { return $x > 0 ? $x : 0; };"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert_eq!(result, r#"<?php $fn = fn($x) => $x > 0 ? $x : 0;"#);
    }

    #[test]
    fn test_closure_returning_method_call() {
        let source = r#"<?php $fn = function($x) { return $x->getValue(); };"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert_eq!(result, r#"<?php $fn = fn($x) => $x->getValue();"#);
    }

    // ==================== Multiple Closures ====================

    #[test]
    fn test_multiple_closures() {
        let source = r#"<?php
$a = function($x) { return $x + 1; };
$b = function($x) { return $x * 2; };
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);

        let result = transform(source);
        assert!(result.contains("fn($x) => $x + 1"));
        assert!(result.contains("fn($x) => $x * 2"));
    }

    #[test]
    fn test_nested_closures() {
        let source = r#"<?php $fn = function($x) { return function($y) { return $y * 2; }; };"#;
        let edits = check_php(source);
        // Both closures should be converted
        assert_eq!(edits.len(), 2);
    }
}
