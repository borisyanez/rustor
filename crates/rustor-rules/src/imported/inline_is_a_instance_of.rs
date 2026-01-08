//! Rule: Change `is_a()` with object and class name check to `instanceof`
//!
//! Example:
//! ```php
//! // Before
//! class SomeClass
//! {
//!     public function run(object $object)
//!     {
//!         return is_a($object, SomeType::class);
//!     }
//! }
//!
//! // After
//! class SomeClass
//! {
//!     public function run(object $object)
//!     {
//!         return $object instanceof SomeType;
//!     }
//! }
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/CodeQuality/Rector/FuncCall/InlineIsAInstanceOfRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_inline_is_a_instance_of<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = InlineIsAInstanceOfVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct InlineIsAInstanceOfVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for InlineIsAInstanceOfVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("is_a") {
                // Need exactly 2 arguments: object and class name
                let args: Vec<_> = call.argument_list.arguments.iter().collect();
                if args.len() >= 2 {
                    let obj_str = &self.source[args[0].span().start.offset as usize..args[0].span().end.offset as usize];
                    let class_str = &self.source[args[1].span().start.offset as usize..args[1].span().end.offset as usize];
                    // Remove ::class suffix if present
                    let class_name = class_str.trim_end_matches("::class");
                    let replacement = format!("{} instanceof {}", obj_str, class_name);

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Replace is_a() with instanceof",
                    ));
                }
            }
        }
        true
    }
}

pub struct InlineIsAInstanceOfRule;

impl Rule for InlineIsAInstanceOfRule {
    fn name(&self) -> &'static str {
        "inline_is_a_instance_of"
    }

    fn description(&self) -> &'static str {
        "Change `is_a()` with object and class name check to `instanceof`"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_inline_is_a_instance_of(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

