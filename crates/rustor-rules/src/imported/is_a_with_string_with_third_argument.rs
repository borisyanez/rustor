//! Rule: Complete missing 3rd argument in case is_a() function in case of strings
//!
//! Example:
//! ```php
//! // Before
//! class SomeClass
//! {
//!     public function __construct(string $value)
//!     {
//!         return is_a($value, 'stdClass');
//!     }
//! }
//!
//! // After
//! class SomeClass
//! {
//!     public function __construct(string $value)
//!     {
//!         return is_a($value, 'stdClass', true);
//!     }
//! }
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/CodeQuality/Rector/FuncCall/IsAWithStringWithThirdArgumentRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_is_a_with_string_with_third_argument<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = IsAWithStringWithThirdArgumentVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct IsAWithStringWithThirdArgumentVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for IsAWithStringWithThirdArgumentVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("is_a") {
                // Get the argument
                if let Some(arg) = call.argument_list.arguments.first() {
                    let arg_str = &self.source[arg.span().start.offset as usize..arg.span().end.offset as usize];
                    let replacement = format!("{} instanceof a", arg_str);

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Replace is_a() with instanceof a comparison",
                    ));
                }
            }
        }
        true
    }
}

pub struct IsAWithStringWithThirdArgumentRule;

impl Rule for IsAWithStringWithThirdArgumentRule {
    fn name(&self) -> &'static str {
        "is_a_with_string_with_third_argument"
    }

    fn description(&self) -> &'static str {
        "Complete missing 3rd argument in case is_a() function in case of strings"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_is_a_with_string_with_third_argument(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

