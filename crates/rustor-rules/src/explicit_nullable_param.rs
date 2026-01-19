//! Rule: Make implicit nullable parameter types explicit (PHP 8.4+)
//!
//! In PHP 8.4, implicitly nullable parameter types are deprecated.
//! This rule converts parameters with typed hints and null defaults to explicit nullable types.
//!
//! Example:
//! ```php
//! // Before (deprecated in PHP 8.4)
//! function foo(string $param = null) {}
//! function bar(int $x = null, array $y = null) {}
//!
//! // After
//! function foo(?string $param = null) {}
//! function bar(?int $x = null, ?array $y = null) {}
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for implicit nullable parameters
pub fn check_explicit_nullable_param<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut checker = ExplicitNullableParamChecker {
        source,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct ExplicitNullableParamChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> ExplicitNullableParamChecker<'s> {
    fn check_program(&mut self, program: &Program<'_>) {
        for stmt in program.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::Function(func) => {
                self.check_parameter_list(&func.parameter_list);
                self.check_block(&func.body);
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    self.check_class_like_member(member);
                }
            }
            Statement::Interface(iface) => {
                for member in iface.members.iter() {
                    self.check_class_like_member(member);
                }
            }
            Statement::Trait(tr) => {
                for member in tr.members.iter() {
                    self.check_class_like_member(member);
                }
            }
            Statement::Namespace(ns) => {
                let statements = match &ns.body {
                    NamespaceBody::Implicit(body) => &body.statements,
                    NamespaceBody::BraceDelimited(body) => &body.statements,
                };
                for inner in statements.iter() {
                    self.check_statement(inner);
                }
            }
            Statement::Block(block) => {
                self.check_block(block);
            }
            Statement::Expression(expr_stmt) => {
                self.check_expression(expr_stmt.expression);
            }
            _ => {}
        }
    }

    fn check_class_like_member(&mut self, member: &ClassLikeMember<'_>) {
        if let ClassLikeMember::Method(method) = member {
            self.check_parameter_list(&method.parameter_list);
            if let MethodBody::Concrete(ref body) = method.body {
                self.check_block(body);
            }
        }
    }

    fn check_block(&mut self, block: &Block<'_>) {
        for stmt in block.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_expression(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::Closure(closure) => {
                self.check_parameter_list(&closure.parameter_list);
            }
            Expression::ArrowFunction(arrow) => {
                self.check_parameter_list(&arrow.parameter_list);
            }
            Expression::Parenthesized(paren) => {
                self.check_expression(paren.expression);
            }
            Expression::Assignment(assign) => {
                self.check_expression(&assign.rhs);
            }
            Expression::Call(call) => match call {
                Call::Function(func) => {
                    for arg in func.argument_list.arguments.iter() {
                        self.check_expression(arg.value());
                    }
                }
                Call::Method(method) => {
                    self.check_expression(&method.object);
                    for arg in method.argument_list.arguments.iter() {
                        self.check_expression(arg.value());
                    }
                }
                Call::NullSafeMethod(method) => {
                    self.check_expression(&method.object);
                    for arg in method.argument_list.arguments.iter() {
                        self.check_expression(arg.value());
                    }
                }
                Call::StaticMethod(static_method) => {
                    for arg in static_method.argument_list.arguments.iter() {
                        self.check_expression(arg.value());
                    }
                }
            },
            _ => {}
        }
    }

    fn check_parameter_list(&mut self, param_list: &FunctionLikeParameterList<'_>) {
        for param in param_list.parameters.nodes.iter() {
            if let Some(edit) = self.check_parameter(param) {
                self.edits.push(edit);
            }
        }
    }

    fn check_parameter(&self, param: &FunctionLikeParameter<'_>) -> Option<Edit> {
        // Must have a type hint
        let hint = param.hint.as_ref()?;

        // Must have a default value of null
        let default = param.default_value.as_ref()?;
        if !self.is_null_literal(&default.value) {
            return None;
        }

        // Check if the type is already nullable (starts with ?)
        let hint_span = hint.span();
        let hint_source =
            &self.source[hint_span.start.offset as usize..hint_span.end.offset as usize];

        // Skip if already nullable
        if hint_source.starts_with('?') {
            return None;
        }

        // Skip union types that include null (e.g., string|null)
        if self.is_union_with_null(hint) {
            return None;
        }

        // Skip if it's already a Nullable hint type (AST level check)
        if matches!(hint, Hint::Nullable(_)) {
            return None;
        }

        // Create edit to add ? before the type
        let replacement = format!("?{}", hint_source);

        Some(Edit::new(
            hint_span,
            replacement,
            "Make implicit nullable parameter explicit (PHP 8.4+)",
        ))
    }

    fn is_null_literal(&self, expr: &Expression<'_>) -> bool {
        if let Expression::Literal(Literal::Null(_)) = expr {
            return true;
        }
        false
    }

    fn is_union_with_null(&self, hint: &Hint<'_>) -> bool {
        if let Hint::Union(union) = hint {
            // Check left side
            if self.hint_is_null(&union.left) {
                return true;
            }
            // Check right side
            if self.hint_is_null(&union.right) {
                return true;
            }
            // Check if left or right are themselves unions with null
            if self.is_union_with_null(&union.left) || self.is_union_with_null(&union.right) {
                return true;
            }
        }
        false
    }

    fn hint_is_null(&self, hint: &Hint<'_>) -> bool {
        let span = hint.span();
        let source = &self.source[span.start.offset as usize..span.end.offset as usize];
        source.eq_ignore_ascii_case("null")
    }
}

pub struct ExplicitNullableParamRule;

impl Rule for ExplicitNullableParamRule {
    fn name(&self) -> &'static str {
        "explicit_nullable_param"
    }

    fn description(&self) -> &'static str {
        "Make implicit nullable parameter types explicit (PHP 8.4+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_explicit_nullable_param(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php84)
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
        check_explicit_nullable_param(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Function Tests ====================

    #[test]
    fn test_basic_string() {
        let source = r#"<?php function foo(string $param = null) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php function foo(?string $param = null) {}"
        );
    }

    #[test]
    fn test_basic_int() {
        let source = r#"<?php function foo(int $x = null) {}"#;
        assert_eq!(transform(source), "<?php function foo(?int $x = null) {}");
    }

    #[test]
    fn test_basic_array() {
        let source = r#"<?php function foo(array $arr = null) {}"#;
        assert_eq!(
            transform(source),
            "<?php function foo(?array $arr = null) {}"
        );
    }

    #[test]
    fn test_basic_class_type() {
        let source = r#"<?php function foo(DateTime $dt = null) {}"#;
        assert_eq!(
            transform(source),
            "<?php function foo(?DateTime $dt = null) {}"
        );
    }

    #[test]
    fn test_namespaced_type() {
        let source = r#"<?php function foo(\App\User $user = null) {}"#;
        assert_eq!(
            transform(source),
            r#"<?php function foo(?\App\User $user = null) {}"#
        );
    }

    // ==================== Multiple Parameters ====================

    #[test]
    fn test_multiple_params() {
        let source = r#"<?php function foo(string $a = null, int $b = null) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        assert_eq!(
            transform(source),
            "<?php function foo(?string $a = null, ?int $b = null) {}"
        );
    }

    #[test]
    fn test_mixed_params() {
        let source = r#"<?php function foo(string $a, int $b = null, array $c) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php function foo(string $a, ?int $b = null, array $c) {}"
        );
    }

    // ==================== Method Tests ====================

    #[test]
    fn test_method() {
        let source = r#"<?php
class Foo {
    public function bar(string $param = null) {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("?string $param = null"));
    }

    #[test]
    fn test_static_method() {
        let source = r#"<?php
class Foo {
    public static function create(array $config = null) {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_constructor() {
        let source = r#"<?php
class User {
    public function __construct(string $name = null, int $age = null) {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Closure/Arrow Function Tests ====================

    #[test]
    fn test_closure() {
        let source = r#"<?php $fn = function(string $x = null) {};"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php $fn = function(?string $x = null) {};"
        );
    }

    #[test]
    fn test_arrow_function() {
        let source = r#"<?php $fn = fn(int $x = null) => $x ?? 0;"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php $fn = fn(?int $x = null) => $x ?? 0;"
        );
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_already_nullable() {
        let source = r#"<?php function foo(?string $param = null) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_default() {
        let source = r#"<?php function foo(string $param) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_null_default() {
        let source = r#"<?php function foo(string $param = 'default') {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_union_with_null() {
        let source = r#"<?php function foo(string|null $param = null) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_type() {
        let source = r#"<?php function foo($param = null) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_int_default() {
        let source = r#"<?php function foo(int $x = 0) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Complex Types ====================

    #[test]
    fn test_callable() {
        let source = r#"<?php function foo(callable $cb = null) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php function foo(?callable $cb = null) {}"
        );
    }

    #[test]
    fn test_iterable() {
        let source = r#"<?php function foo(iterable $items = null) {}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php function foo(?iterable $items = null) {}"
        );
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_multiple_functions() {
        let source = r#"<?php
function foo(string $a = null) {}
function bar(int $b = null) {}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_interface_method() {
        let source = r#"<?php
interface Foo {
    public function bar(string $param = null);
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_trait_method() {
        let source = r#"<?php
trait Foo {
    public function bar(string $param = null) {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
