//! Rule: Convert Closure::fromCallable() to first-class callable syntax (PHP 8.1+)
//!
//! Example:
//! ```php
//! // Before
//! $fn = Closure::fromCallable([$this, 'method']);
//! $fn = Closure::fromCallable([self::class, 'method']);
//! $fn = Closure::fromCallable('strlen');
//! $fn = Closure::fromCallable([$obj, 'method']);
//!
//! // After
//! $fn = $this->method(...);
//! $fn = self::method(...);
//! $fn = strlen(...);
//! $fn = $obj->method(...);
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use mago_syntax::ast::access::Access;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for Closure::fromCallable() calls
pub fn check_first_class_callables<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = FirstClassCallablesVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct FirstClassCallablesVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for FirstClassCallablesVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::StaticMethod(static_call)) = expr {
            self.check_static_method_call(static_call);
        }
        true // Continue traversal
    }
}

impl<'s> FirstClassCallablesVisitor<'s> {
    fn check_static_method_call(&mut self, call: &StaticMethodCall<'_>) {
        // Check if it's Closure::fromCallable
        let class_name = self.get_class_name(&call.class);
        if !class_name.eq_ignore_ascii_case("Closure") {
            return;
        }

        let method_name = self.get_method_name(&call.method);
        if !method_name.eq_ignore_ascii_case("fromCallable") {
            return;
        }

        // Must have exactly 1 argument
        let args: Vec<_> = call.argument_list.arguments.iter().collect();
        if args.len() != 1 {
            return;
        }

        // Get the argument
        let arg = &args[0];
        if arg.is_unpacked() {
            return;
        }

        let arg_expr = arg.value();

        // Try to convert the argument to first-class callable syntax
        if let Some(replacement) = self.convert_to_first_class(arg_expr) {
            let span = call.span();
            self.edits.push(Edit::new(
                span,
                replacement,
                "Convert Closure::fromCallable() to first-class callable syntax (PHP 8.1+)",
            ));
        }
    }

    fn get_class_name(&self, class: &Expression<'_>) -> String {
        match class {
            Expression::Identifier(ident) => {
                let span = ident.span();
                self.source[span.start.offset as usize..span.end.offset as usize].to_string()
            }
            _ => String::new(),
        }
    }

    fn get_method_name(&self, method: &ClassLikeMemberSelector<'_>) -> String {
        match method {
            ClassLikeMemberSelector::Identifier(ident) => {
                let span = ident.span();
                self.source[span.start.offset as usize..span.end.offset as usize].to_string()
            }
            _ => String::new(),
        }
    }

    fn convert_to_first_class(&self, expr: &Expression<'_>) -> Option<String> {
        match expr {
            // String callable: 'strlen' -> strlen(...)
            Expression::Literal(Literal::String(string_lit)) => {
                let span = string_lit.span();
                let raw = &self.source[span.start.offset as usize..span.end.offset as usize];
                // Remove quotes
                let func_name = raw.trim_matches(|c| c == '\'' || c == '"');
                // Validate it looks like a function name (no :: or ->)
                if func_name.contains("::") || func_name.contains("->") {
                    return None;
                }
                Some(format!("{}(...)", func_name))
            }

            // Array callable: [$obj, 'method'] or [self::class, 'method']
            Expression::Array(array) => {
                self.convert_array_callable(array)
            }

            Expression::LegacyArray(array) => {
                self.convert_legacy_array_callable(array)
            }

            _ => None,
        }
    }

    fn convert_array_callable(&self, array: &Array<'_>) -> Option<String> {
        let elements: Vec<_> = array.elements.iter().collect();
        if elements.len() != 2 {
            return None;
        }

        // First element: object/class
        let obj = match &elements[0] {
            ArrayElement::Value(val) => val.value,
            _ => return None,
        };

        // Second element: method name (must be string)
        let method = match &elements[1] {
            ArrayElement::Value(val) => {
                self.extract_string_value(val.value)?
            }
            _ => return None,
        };

        self.format_callable(obj, &method)
    }

    fn convert_legacy_array_callable(&self, array: &LegacyArray<'_>) -> Option<String> {
        let elements: Vec<_> = array.elements.iter().collect();
        if elements.len() != 2 {
            return None;
        }

        // First element: object/class
        let obj = match &elements[0] {
            ArrayElement::Value(val) => val.value,
            _ => return None,
        };

        // Second element: method name (must be string)
        let method = match &elements[1] {
            ArrayElement::Value(val) => {
                self.extract_string_value(val.value)?
            }
            _ => return None,
        };

        self.format_callable(obj, &method)
    }

    fn extract_string_value(&self, expr: &Expression<'_>) -> Option<String> {
        if let Expression::Literal(Literal::String(string_lit)) = expr {
            let span = string_lit.span();
            let raw = &self.source[span.start.offset as usize..span.end.offset as usize];
            Some(raw.trim_matches(|c| c == '\'' || c == '"').to_string())
        } else {
            None
        }
    }

    fn format_callable(&self, obj: &Expression<'_>, method: &str) -> Option<String> {
        match obj {
            // $this->method(...)
            Expression::Variable(var) => {
                let span = var.span();
                let var_name = &self.source[span.start.offset as usize..span.end.offset as usize];
                Some(format!("{}->{}(...)", var_name, method))
            }

            // self::method(...) or static::method(...) or ClassName::method(...)
            Expression::Identifier(ident) => {
                let span = ident.span();
                let class_name = &self.source[span.start.offset as usize..span.end.offset as usize];
                Some(format!("{}::{}(...)", class_name, method))
            }

            // SomeClass::class -> SomeClass::method(...)
            Expression::Access(Access::ClassConstant(cc)) => {
                // Check if it's ::class
                let const_name = match &cc.constant {
                    ClassLikeConstantSelector::Identifier(ident) => {
                        let span = ident.span();
                        &self.source[span.start.offset as usize..span.end.offset as usize]
                    }
                    _ => return None,
                };

                if const_name.eq_ignore_ascii_case("class") {
                    let class_span = cc.class.span();
                    let class_name = &self.source[class_span.start.offset as usize..class_span.end.offset as usize];
                    Some(format!("{}::{}(...)", class_name, method))
                } else {
                    None
                }
            }

            _ => None,
        }
    }
}

pub struct FirstClassCallablesRule;

impl Rule for FirstClassCallablesRule {
    fn name(&self) -> &'static str {
        "first_class_callables"
    }

    fn description(&self) -> &'static str {
        "Convert Closure::fromCallable() to first-class callable syntax"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_first_class_callables(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php81)
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
        check_first_class_callables(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== String Callables ====================

    #[test]
    fn test_string_callable() {
        let source = r#"<?php
$fn = Closure::fromCallable('strlen');
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("strlen(...)"));
    }

    #[test]
    fn test_string_callable_double_quotes() {
        let source = r#"<?php
$fn = Closure::fromCallable("array_map");
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("array_map(...)"));
    }

    // ==================== Array Callables ====================

    #[test]
    fn test_array_callable_this() {
        let source = r#"<?php
$fn = Closure::fromCallable([$this, 'method']);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$this->method(...)"));
    }

    #[test]
    fn test_array_callable_variable() {
        let source = r#"<?php
$fn = Closure::fromCallable([$obj, 'doSomething']);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$obj->doSomething(...)"));
    }

    #[test]
    fn test_array_callable_self() {
        let source = r#"<?php
$fn = Closure::fromCallable([self::class, 'staticMethod']);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("self::staticMethod(...)"));
    }

    #[test]
    fn test_array_callable_static() {
        let source = r#"<?php
$fn = Closure::fromCallable([static::class, 'method']);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("static::method(...)"));
    }

    #[test]
    fn test_array_callable_class_name() {
        let source = r#"<?php
$fn = Closure::fromCallable([MyClass::class, 'factory']);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("MyClass::factory(...)"));
    }

    // ==================== Legacy Array Syntax ====================

    #[test]
    fn test_legacy_array_callable() {
        let source = r#"<?php
$fn = Closure::fromCallable(array($this, 'method'));
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("$this->method(...)"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_non_closure() {
        let source = r#"<?php
$fn = SomeClass::fromCallable('test');
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_non_from_callable() {
        let source = r#"<?php
$fn = Closure::bind($closure, $newThis);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_multiple_args() {
        let source = r#"<?php
$fn = Closure::fromCallable('test', 'extra');
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_callables() {
        let source = r#"<?php
$a = Closure::fromCallable('strlen');
$b = Closure::fromCallable([$this, 'method']);
$c = Closure::fromCallable([self::class, 'static']);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 3);
        let result = transform(source);
        assert!(result.contains("strlen(...)"));
        assert!(result.contains("$this->method(...)"));
        assert!(result.contains("self::static(...)"));
    }
}
