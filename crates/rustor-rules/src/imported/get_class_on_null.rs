//! Rule: Null is no more allowed in `get_class()`
//!
//! Example:
//! ```php
//! // Before
//! final class SomeClass
//! {
//!     public function getItem()
//!     {
//!         $value = null;
//!         return get_class($value);
//!     }
//! }
//!
//! // After
//! final class SomeClass
//! {
//!     public function getItem()
//!     {
//!         $value = null;
//!         return $value !== null ? get_class($value) : self::class;
//!     }
//! }
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php72/Rector/FuncCall/GetClassOnNullRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_get_class_on_null<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = GetClassOnNullVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct GetClassOnNullVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for GetClassOnNullVisitor<'s> {
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

pub struct GetClassOnNullRule;

impl Rule for GetClassOnNullRule {
    fn name(&self) -> &'static str {
        "get_class_on_null"
    }

    fn description(&self) -> &'static str {
        "Null is no more allowed in `get_class()`"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_get_class_on_null(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php72)
    }
}

