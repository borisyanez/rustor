//! Rule: Change get_class($object) to faster $object::class
//!
//! Example:
//! ```php
//! // Before
//! class SomeClass
//! {
//!     public function run($object)
//!     {
//!         return get_class($object);
//!     }
//! }
//!
//! // After
//! class SomeClass
//! {
//!     public function run($object)
//!     {
//!         return $object::class;
//!     }
//! }
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php80/Rector/FuncCall/ClassOnObjectRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_class_on_object<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ClassOnObjectVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ClassOnObjectVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ClassOnObjectVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("get_class") {
                // Get the argument
                if let Some(arg) = call.argument_list.arguments.first() {
                    let arg_str = &self.source[arg.span().start.offset as usize..arg.span().end.offset as usize];
                    let replacement = format!("{}::class", arg_str);

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Replace get_class() with ::class constant",
                    ));
                }
            }
        }
        true
    }
}

pub struct ClassOnObjectRule;

impl Rule for ClassOnObjectRule {
    fn name(&self) -> &'static str {
        "class_on_object"
    }

    fn description(&self) -> &'static str {
        "Change get_class($object) to faster $object::class"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_class_on_object(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
    }
}

