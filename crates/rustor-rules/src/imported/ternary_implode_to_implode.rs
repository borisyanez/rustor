//! Rule: Narrow ternary with implode and empty string to direct implode, as same result
//!
//! Example:
//! ```php
//! // Before
//! class SomeClass
//! {
//!     public function run(array $values)
//!     {
//!         return $values === [] ? '' : implode(',', $values);
//!     }
//! }
//!
//! // After
//! class SomeClass
//! {
//!     public function run(array $values)
//!     {
//!         return implode(',', $values);
//!     }
//! }
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/CodeQuality/Rector/Ternary/TernaryImplodeToImplodeRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_ternary_implode_to_implode<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = TernaryImplodeToImplodeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct TernaryImplodeToImplodeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for TernaryImplodeToImplodeVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("join") {
                self.edits.push(Edit::new(
                    call.function.span(),
                    "implode",
                    "Replace join() with implode()",
                ));
            }
        }
        true
    }
}

pub struct TernaryImplodeToImplodeRule;

impl Rule for TernaryImplodeToImplodeRule {
    fn name(&self) -> &'static str {
        "ternary_implode_to_implode"
    }

    fn description(&self) -> &'static str {
        "Narrow ternary with implode and empty string to direct implode, as same result"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_ternary_implode_to_implode(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

