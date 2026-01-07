//! Rule: Convert legacy PHP 4-style constructors to __construct
//!
//! PHP 4-style constructors (methods with the same name as the class) were
//! deprecated in PHP 7.0 and removed in PHP 8.0.
//!
//! Example:
//! ```php
//! // Before
//! class Foo {
//!     function Foo($x) { $this->x = $x; }
//! }
//!
//! // After
//! class Foo {
//!     function __construct($x) { $this->x = $x; }
//! }
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use mago_syntax::ast::Sequence;
use rustor_core::Edit;

/// Check a parsed PHP program for legacy constructor patterns
pub fn check_class_constructor<'a>(program: &Program<'a>, _source: &str) -> Vec<Edit> {
    let mut edits = Vec::new();

    for stmt in program.statements.iter() {
        check_statement(stmt, &mut edits);
    }

    edits
}

fn check_statement<'a>(stmt: &Statement<'a>, edits: &mut Vec<Edit>) {
    match stmt {
        Statement::Class(class) => {
            check_class(class, edits);
        }
        Statement::Namespace(ns) => {
            let statements = match &ns.body {
                NamespaceBody::Implicit(body) => &body.statements,
                NamespaceBody::BraceDelimited(body) => &body.statements,
            };
            for inner in statements.iter() {
                check_statement(inner, edits);
            }
        }
        Statement::Block(block) => {
            for inner in block.statements.iter() {
                check_statement(inner, edits);
            }
        }
        Statement::If(if_stmt) => {
            check_if_body(&if_stmt.body, edits);
        }
        _ => {}
    }
}

fn check_if_body<'a>(body: &IfBody<'a>, edits: &mut Vec<Edit>) {
    match body {
        IfBody::Statement(stmt_body) => {
            check_statement(stmt_body.statement, edits);
            for else_if in stmt_body.else_if_clauses.iter() {
                check_statement(else_if.statement, edits);
            }
            if let Some(else_clause) = &stmt_body.else_clause {
                check_statement(else_clause.statement, edits);
            }
        }
        IfBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                check_statement(inner, edits);
            }
            for else_if in block.else_if_clauses.iter() {
                for inner in else_if.statements.iter() {
                    check_statement(inner, edits);
                }
            }
            if let Some(else_clause) = &block.else_clause {
                for inner in else_clause.statements.iter() {
                    check_statement(inner, edits);
                }
            }
        }
    }
}

fn check_class<'a>(class: &Class<'a>, edits: &mut Vec<Edit>) {
    let class_name = class.name.value;

    // Check if class already has a __construct method
    let has_construct = class.members.iter().any(|member| {
        if let ClassLikeMember::Method(method) = member {
            method.name.value.eq_ignore_ascii_case("__construct")
        } else {
            false
        }
    });

    // If class already has __construct, don't convert legacy constructor
    // (it would cause a conflict)
    if has_construct {
        return;
    }

    // Look for methods with the same name as the class (case-insensitive)
    for member in class.members.iter() {
        if let ClassLikeMember::Method(method) = member {
            // PHP is case-insensitive for class/method names
            if method.name.value.eq_ignore_ascii_case(class_name) {
                // Skip if method has a return type hint (constructors can't have return types)
                if method.return_type_hint.is_some() {
                    continue;
                }

                // Skip if method body returns a value (constructors don't return values)
                if let MethodBody::Concrete(body) = &method.body {
                    if has_return_with_value(&body.statements) {
                        continue;
                    }
                }

                // Replace just the method name with __construct
                let name_span = method.name.span();
                edits.push(Edit::new(
                    name_span,
                    "__construct".to_string(),
                    "Replace legacy constructor with __construct",
                ));
            }
        }
    }
}

/// Check if any statement in the list contains a return with a value
fn has_return_with_value(statements: &Sequence<'_, Statement<'_>>) -> bool {
    for stmt in statements.iter() {
        if check_statement_for_return_value(stmt) {
            return true;
        }
    }
    false
}

/// Recursively check a statement for return statements with values
fn check_statement_for_return_value(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::Return(ret) => ret.value.is_some(),
        Statement::Block(block) => {
            for inner in block.statements.iter() {
                if check_statement_for_return_value(inner) {
                    return true;
                }
            }
            false
        }
        Statement::If(if_stmt) => check_if_for_return_value(&if_stmt.body),
        Statement::Try(try_stmt) => {
            for inner in try_stmt.block.statements.iter() {
                if check_statement_for_return_value(inner) {
                    return true;
                }
            }
            for catch in try_stmt.catch_clauses.iter() {
                for inner in catch.block.statements.iter() {
                    if check_statement_for_return_value(inner) {
                        return true;
                    }
                }
            }
            if let Some(finally) = &try_stmt.finally_clause {
                for inner in finally.block.statements.iter() {
                    if check_statement_for_return_value(inner) {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// Check if body for return statements with values
fn check_if_for_return_value(body: &IfBody<'_>) -> bool {
    match body {
        IfBody::Statement(stmt_body) => {
            if check_statement_for_return_value(stmt_body.statement) {
                return true;
            }
            for else_if in stmt_body.else_if_clauses.iter() {
                if check_statement_for_return_value(else_if.statement) {
                    return true;
                }
            }
            if let Some(else_clause) = &stmt_body.else_clause {
                if check_statement_for_return_value(else_clause.statement) {
                    return true;
                }
            }
            false
        }
        IfBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                if check_statement_for_return_value(inner) {
                    return true;
                }
            }
            for else_if in block.else_if_clauses.iter() {
                for inner in else_if.statements.iter() {
                    if check_statement_for_return_value(inner) {
                        return true;
                    }
                }
            }
            if let Some(else_clause) = &block.else_clause {
                for inner in else_clause.statements.iter() {
                    if check_statement_for_return_value(inner) {
                        return true;
                    }
                }
            }
            false
        }
    }
}

use crate::registry::{Category, Rule};

pub struct ClassConstructorRule;

impl Rule for ClassConstructorRule {
    fn name(&self) -> &'static str {
        "class_constructor"
    }

    fn description(&self) -> &'static str {
        "Convert legacy ClassName() constructor to __construct()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_class_constructor(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
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
        check_class_constructor(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_legacy_constructor() {
        let source = r#"<?php
class Foo {
    function Foo() {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("function __construct()"));
        assert!(!result.contains("function Foo()"));
    }

    #[test]
    fn test_legacy_constructor_with_params() {
        let source = r#"<?php
class User {
    function User($name, $email) {
        $this->name = $name;
        $this->email = $email;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("function __construct($name, $email)"));
    }

    #[test]
    fn test_legacy_constructor_with_visibility() {
        let source = r#"<?php
class Service {
    public function Service() {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("public function __construct()"));
    }

    #[test]
    fn test_case_insensitive_match() {
        // PHP constructor matching is case-insensitive
        let source = r#"<?php
class foo {
    function FOO() {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("function __construct()"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_already_has_construct() {
        // If class already has __construct, don't convert legacy constructor
        let source = r#"<?php
class Foo {
    function __construct() {}
    function Foo() { /* old code */ }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_regular_method() {
        let source = r#"<?php
class Foo {
    function bar() {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_modern_constructor() {
        let source = r#"<?php
class Foo {
    function __construct() {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_different_name() {
        // Method name must match class name
        let source = r#"<?php
class Foo {
    function Bar() {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_with_return_type() {
        // Methods with return type hints can't be constructors
        let source = r#"<?php
class Foo {
    function Foo(): void {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_returns_value() {
        // Methods that return values can't be constructors
        let source = r#"<?php
class EloquentBuilderTestWhereBelongsToStub {
    public function eloquentBuilderTestWhereBelongsToStub() {
        return $this->belongsTo(self::class);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_returns_value_in_if() {
        // Methods that return values in conditionals can't be constructors
        let source = r#"<?php
class Foo {
    function Foo($x) {
        if ($x) {
            return $x;
        }
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_allow_method_with_empty_return() {
        // Methods with empty return statements can still be constructors
        let source = r#"<?php
class Foo {
    function Foo($x) {
        if (!$x) {
            return;
        }
        $this->x = $x;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple Classes ====================

    #[test]
    fn test_multiple_classes() {
        let source = r#"<?php
class Foo {
    function Foo() {}
}

class Bar {
    function Bar($x) {}
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        let result = transform(source);
        assert!(result.contains("function __construct()"));
        assert!(result.contains("function __construct($x)"));
    }

    // ==================== Namespaced Classes ====================

    #[test]
    fn test_namespaced_class() {
        let source = r#"<?php
namespace App\Services;

class Logger {
    function Logger($path) {
        $this->path = $path;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("function __construct($path)"));
    }

    #[test]
    fn test_braced_namespace() {
        let source = r#"<?php
namespace App {
    class Cache {
        function Cache() {}
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("function __construct()"));
    }
}
