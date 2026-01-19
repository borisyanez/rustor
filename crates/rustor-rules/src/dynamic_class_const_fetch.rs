//! Rule: Convert constant() to dynamic class const fetch
//!
//! Since PHP 8.3, dynamic class constant fetch is supported.
//!
//! Transformation:
//! - `constant(Example::class . '::' . $name)` → `Example::{$name}`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for constant() calls that can use dynamic class const fetch
pub fn check_dynamic_class_const_fetch<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = DynamicClassConstFetchVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct DynamicClassConstFetchVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for DynamicClassConstFetchVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_convert_constant_call(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to convert constant(Class::class . '::' . $name) to Class::{$name}
fn try_convert_constant_call(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name is "constant"
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("constant") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Must have exactly 1 argument
    if args.len() != 1 {
        return None;
    }

    let arg = args[0].value();

    // The argument must be a concatenation: (Class::class . '::') . $name
    // This is nested: left is (Class::class . '::'), right is $name
    let outer_concat = if let Expression::Binary(binary) = arg {
        if matches!(binary.operator, BinaryOperator::StringConcat(_)) {
            binary
        } else {
            return None;
        }
    } else {
        return None;
    };

    // Left side must be another concatenation: Class::class . '::'
    let inner_concat = if let Expression::Binary(binary) = &*outer_concat.lhs {
        if matches!(binary.operator, BinaryOperator::StringConcat(_)) {
            binary
        } else {
            return None;
        }
    } else {
        return None;
    };

    // Inner left must be Class::class (class constant fetch)
    let class_const = if let Expression::Access(Access::ClassConstant(cc)) = &*inner_concat.lhs {
        cc
    } else {
        return None;
    };

    // The constant name must be "class"
    let const_name = match &class_const.constant {
        ClassLikeConstantSelector::Identifier(ident) => {
            let span = ident.span();
            &source[span.start.offset as usize..span.end.offset as usize]
        }
        _ => return None,
    };

    if const_name.to_ascii_lowercase() != "class" {
        return None;
    }

    // Get the class name
    let class_span = class_const.class.span();
    let class_name = &source[class_span.start.offset as usize..class_span.end.offset as usize];

    // Inner right must be '::' string literal
    if let Expression::Literal(Literal::String(string_lit)) = &*inner_concat.rhs {
        let str_span = string_lit.span();
        let str_text = &source[str_span.start.offset as usize..str_span.end.offset as usize];
        // Check if it's '::'  (including quotes)
        if !str_text.contains("::") {
            return None;
        }
    } else {
        return None;
    }

    // Outer right is the dynamic part ($name, etc.)
    let dynamic_span = outer_concat.rhs.span();
    let dynamic_part = &source[dynamic_span.start.offset as usize..dynamic_span.end.offset as usize];

    // Build replacement: Class::{$dynamic}
    let func_span = func_call.span();
    let replacement = format!("{}::{{{}}}", class_name, dynamic_part);

    Some(Edit::new(
        func_span,
        replacement,
        "Convert constant() to dynamic class const fetch",
    ))
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct DynamicClassConstFetchRule;

impl Rule for DynamicClassConstFetchRule {
    fn name(&self) -> &'static str {
        "dynamic_class_const_fetch"
    }

    fn description(&self) -> &'static str {
        "Convert constant() to dynamic class const fetch"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_dynamic_class_const_fetch(program, source)
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
        check_dynamic_class_const_fetch(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php constant(Example::class . '::' . $constName);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php Example::{$constName};");
    }

    #[test]
    fn test_with_fqcn() {
        let source = r"<?php constant(\App\Example::class . '::' . $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), r"<?php \App\Example::{$name};");
    }

    #[test]
    fn test_with_self() {
        let source = "<?php constant(self::class . '::' . $name);";
        assert_eq!(transform(source), "<?php self::{$name};");
    }

    #[test]
    fn test_with_static() {
        let source = "<?php constant(static::class . '::' . $name);";
        assert_eq!(transform(source), "<?php static::{$name};");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $value = constant(Foo::class . '::' . $const);";
        assert_eq!(transform(source), "<?php $value = Foo::{$const};");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return constant(Bar::class . '::' . $name);";
        assert_eq!(transform(source), "<?php return Bar::{$name};");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = constant(Foo::class . '::' . $x);
$b = constant(Bar::class . '::' . $y);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_not_class_const() {
        // Not using ::class, just a string
        let source = "<?php constant('Foo' . '::' . $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_wrong_separator() {
        // Using wrong separator
        let source = "<?php constant(Foo::class . '_' . $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_args() {
        let source = "<?php constant();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_simple_string() {
        // Just a simple string argument
        let source = "<?php constant('SOME_CONST');";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php defined(Foo::class . '::' . $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
