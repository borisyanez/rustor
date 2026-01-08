//! Rule: Add #[Override] attribute to methods that override parent methods (PHP 8.3+)
//!
//! Example:
//! ```php
//! // Before
//! class Child extends Parent {
//!     public function doSomething() {
//!         parent::doSomething();
//!     }
//! }
//!
//! // After
//! class Child extends Parent {
//!     #[Override]
//!     public function doSomething() {
//!         parent::doSomething();
//!     }
//! }
//! ```
//!
//! Detection heuristics (without cross-file analysis):
//! 1. Methods that call `parent::methodName()` where names match
//! 2. `__construct` in classes extending another class
//! 3. Well-known interface method names (Countable, Iterator, ArrayAccess, etc.)

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Well-known interface methods that are commonly implemented
const INTERFACE_METHODS: &[&str] = &[
    // Countable
    "count",
    // Iterator
    "current",
    "key",
    "next",
    "rewind",
    "valid",
    // ArrayAccess
    "offsetExists",
    "offsetGet",
    "offsetSet",
    "offsetUnset",
    // Stringable
    "__toString",
    // JsonSerializable
    "jsonSerialize",
    // IteratorAggregate
    "getIterator",
    // Serializable (deprecated but still common)
    "serialize",
    "unserialize",
];

/// Check a parsed PHP program for methods that should have #[Override]
pub fn check_override_attribute<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = OverrideAttributeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct OverrideAttributeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for OverrideAttributeVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        if let Statement::Class(class) = stmt {
            // Only process classes that extend another class or implement interfaces
            let has_extends = class.extends.is_some();
            let has_implements = class.implements.is_some();

            if has_extends || has_implements {
                self.check_class_methods(class);
            }
        }
        true
    }
}

impl<'s> OverrideAttributeVisitor<'s> {
    fn check_class_methods(&mut self, class: &Class<'_>) {
        let has_extends = class.extends.is_some();

        for member in class.members.iter() {
            if let ClassLikeMember::Method(method) = member {
                // Skip if already has #[Override] attribute
                if self.has_override_attribute(method) {
                    continue;
                }

                // Skip abstract methods (they can't override)
                if method.modifiers.get_abstract().is_some() {
                    continue;
                }

                // Skip private methods (can't override)
                if method.modifiers.get_private().is_some() {
                    continue;
                }

                let method_name = method.name.value;

                // Check if this is likely an override
                let is_override = self.should_add_override(method, method_name, has_extends);

                if is_override {
                    self.add_override_attribute(method);
                }
            }
        }
    }

    fn has_override_attribute(&self, method: &Method<'_>) -> bool {
        for attr_list in method.attribute_lists.iter() {
            for attr in attr_list.attributes.nodes.iter() {
                // Check if the attribute is "Override"
                let name = self.get_attribute_name(attr);
                if name.eq_ignore_ascii_case("override") {
                    return true;
                }
            }
        }
        false
    }

    fn get_attribute_name(&self, attr: &Attribute<'_>) -> String {
        // Extract attribute name from source using span
        let attr_span = attr.name.span();
        let name = &self.source[attr_span.start.offset as usize..attr_span.end.offset as usize];
        // Get just the last part (after any \)
        name.rsplit('\\').next().unwrap_or(name).to_string()
    }

    fn should_add_override(
        &self,
        method: &Method<'_>,
        method_name: &str,
        has_extends: bool,
    ) -> bool {
        // Constructor in a class that extends another
        if has_extends && method_name.eq_ignore_ascii_case("__construct") {
            // Only if it actually calls parent::__construct
            if self.calls_parent_method(method, "__construct") {
                return true;
            }
        }

        // Methods that call parent::methodName()
        if has_extends && self.calls_parent_method(method, method_name) {
            return true;
        }

        // Well-known interface methods
        if INTERFACE_METHODS.iter().any(|m| m.eq_ignore_ascii_case(method_name)) {
            return true;
        }

        false
    }

    fn calls_parent_method(&self, method: &Method<'_>, expected_name: &str) -> bool {
        if let MethodBody::Concrete(body) = &method.body {
            for stmt in body.statements.iter() {
                if self.statement_calls_parent(stmt, expected_name) {
                    return true;
                }
            }
        }
        false
    }

    fn statement_calls_parent(&self, stmt: &Statement<'_>, expected_name: &str) -> bool {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.expression_calls_parent(&expr_stmt.expression, expected_name)
            }
            Statement::Return(ret) => {
                if let Some(expr) = &ret.value {
                    self.expression_calls_parent(expr, expected_name)
                } else {
                    false
                }
            }
            Statement::If(if_stmt) => {
                // Check condition
                if self.expression_calls_parent(&if_stmt.condition, expected_name) {
                    return true;
                }
                // Check body
                match &if_stmt.body {
                    IfBody::Statement(body) => {
                        if self.statement_calls_parent(&body.statement, expected_name) {
                            return true;
                        }
                    }
                    IfBody::ColonDelimited(body) => {
                        for s in body.statements.iter() {
                            if self.statement_calls_parent(s, expected_name) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            _ => false, // We could be more thorough, but this covers common cases
        }
    }

    fn expression_calls_parent(&self, expr: &Expression<'_>, expected_name: &str) -> bool {
        match expr {
            Expression::Call(call) => {
                if let Call::StaticMethod(static_call) = call {
                    // Check if it's parent::methodName()
                    if let Expression::Parent(_) = &*static_call.class {
                        if let ClassLikeMemberSelector::Identifier(id) = &static_call.method {
                            if id.value.eq_ignore_ascii_case(expected_name) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            Expression::Parenthesized(paren) => {
                self.expression_calls_parent(&paren.expression, expected_name)
            }
            Expression::Binary(binop) => {
                self.expression_calls_parent(&binop.lhs, expected_name)
                    || self.expression_calls_parent(&binop.rhs, expected_name)
            }
            Expression::UnaryPostfix(unary) => {
                self.expression_calls_parent(&unary.operand, expected_name)
            }
            Expression::UnaryPrefix(unary) => {
                self.expression_calls_parent(&unary.operand, expected_name)
            }
            Expression::Assignment(assign) => {
                self.expression_calls_parent(&assign.lhs, expected_name)
                    || self.expression_calls_parent(&assign.rhs, expected_name)
            }
            Expression::Conditional(cond) => {
                self.expression_calls_parent(&cond.condition, expected_name)
                    || cond
                        .then
                        .as_ref()
                        .map(|e| self.expression_calls_parent(e, expected_name))
                        .unwrap_or(false)
                    || self.expression_calls_parent(&cond.r#else, expected_name)
            }
            _ => false,
        }
    }

    fn add_override_attribute(&mut self, method: &Method<'_>) {
        // Get the indentation from the method's position
        let method_span = method.span();
        let method_start = method_span.start.offset as usize;

        // Find the start of the line to get indentation
        let line_start = self.source[..method_start]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let indent = &self.source[line_start..method_start];

        // If method already has attributes, add Override to the first attribute list
        if !method.attribute_lists.is_empty() {
            // Add #[Override] before existing attributes
            let first_attr = method.attribute_lists.first().unwrap();
            let attr_span = first_attr.span();

            let new_text = format!("#[Override]\n{}", indent);
            self.edits.push(Edit::new(
                mago_span::Span::new(
                    attr_span.file_id,
                    attr_span.start,
                    attr_span.start, // Insert at start
                ),
                new_text,
                "Add #[Override] attribute (PHP 8.3+)",
            ));
        } else {
            // Add #[Override] before the method modifiers/function keyword
            let insert_span = if let Some(first_modifier) = method.modifiers.iter().next() {
                // Insert before first modifier
                first_modifier.span()
            } else {
                // Insert before function keyword
                method.function.span()
            };

            let new_text = format!("#[Override]\n{}", indent);
            self.edits.push(Edit::new(
                mago_span::Span::new(
                    insert_span.file_id,
                    insert_span.start,
                    insert_span.start, // Insert at start
                ),
                new_text,
                "Add #[Override] attribute (PHP 8.3+)",
            ));
        }
    }
}

pub struct OverrideAttributeRule;

impl Rule for OverrideAttributeRule {
    fn name(&self) -> &'static str {
        "override_attribute"
    }

    fn description(&self) -> &'static str {
        "Add #[Override] attribute to methods that override parent methods"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_override_attribute(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php83)
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
        check_override_attribute(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Tests ====================

    #[test]
    fn test_rule_exists() {
        let rule = OverrideAttributeRule;
        assert_eq!(rule.name(), "override_attribute");
        assert_eq!(rule.min_php_version(), Some(PhpVersion::Php83));
    }

    #[test]
    fn test_method_calls_parent() {
        let source = r#"<?php
class Child extends Parent {
    public function doSomething() {
        parent::doSomething();
    }
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert!(result.contains("#[Override]"));
        assert!(result.contains("#[Override]\n    public function doSomething()"));
    }

    #[test]
    fn test_constructor_calls_parent() {
        let source = r#"<?php
class Child extends Parent {
    public function __construct() {
        parent::__construct();
    }
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert!(result.contains("#[Override]"));
    }

    #[test]
    fn test_interface_method_count() {
        let source = r#"<?php
class MyCollection implements Countable {
    public function count(): int {
        return 0;
    }
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert!(result.contains("#[Override]"));
    }

    #[test]
    fn test_interface_method_iterator() {
        let source = r#"<?php
class MyIterator implements Iterator {
    public function current() { return null; }
    public function key() { return 0; }
    public function next() {}
    public function rewind() {}
    public function valid() { return false; }
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 5);
    }

    #[test]
    fn test_interface_method_arrayaccess() {
        let source = r#"<?php
class MyArray implements ArrayAccess {
    public function offsetExists($offset): bool { return false; }
    public function offsetGet($offset) { return null; }
    public function offsetSet($offset, $value): void {}
    public function offsetUnset($offset): void {}
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 4);
    }

    #[test]
    fn test_to_string_method() {
        let source = r#"<?php
class User implements Stringable {
    public function __toString(): string {
        return 'User';
    }
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_already_has_override() {
        let source = r#"<?php
class Child extends Parent {
    #[Override]
    public function doSomething() {
        parent::doSomething();
    }
}"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip methods that already have #[Override]");
    }

    #[test]
    fn test_skip_private_method() {
        let source = r#"<?php
class Child extends Parent {
    private function helper() {
        parent::helper();
    }
}"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Private methods cannot override");
    }

    #[test]
    fn test_skip_abstract_method() {
        let source = r#"<?php
abstract class Child extends Parent {
    abstract public function doSomething();
}"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Abstract methods cannot override");
    }

    #[test]
    fn test_skip_class_without_extends() {
        let source = r#"<?php
class Standalone {
    public function doSomething() {
        // No parent::doSomething()
    }
}"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip classes without extends/implements");
    }

    #[test]
    fn test_skip_method_not_calling_parent() {
        let source = r#"<?php
class Child extends Parent {
    public function newMethod() {
        // This doesn't call parent::newMethod()
        return 42;
    }
}"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip methods not calling parent with same name");
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_parent_call_in_return() {
        let source = r#"<?php
class Child extends Parent {
    public function getValue() {
        return parent::getValue() * 2;
    }
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_parent_call_in_if() {
        let source = r#"<?php
class Child extends Parent {
    public function check() {
        if (parent::check()) {
            return true;
        }
        return false;
    }
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_method_with_existing_attribute() {
        let source = r#"<?php
class Child extends Parent {
    #[Deprecated]
    public function oldMethod() {
        parent::oldMethod();
    }
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert!(result.contains("#[Override]"));
        assert!(result.contains("#[Deprecated]"));
    }

    #[test]
    fn test_json_serialize() {
        let source = r#"<?php
class User implements JsonSerializable {
    public function jsonSerialize(): array {
        return ['id' => $this->id];
    }
}"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert!(result.contains("#[Override]"));
    }
}
