//! Rule: Changes various implode forms to consistent one
//!
//! Example:
//! ```php
//! // Before
//! class SomeClass
//! {
//!     public function run(array $items)
//!     {
//!         $itemsAsStrings = implode($items);
//!         $itemsAsStrings = implode($items, '|');
//!     }
//! }
//!
//! // After
//! class SomeClass
//! {
//!     public function run(array $items)
//!     {
//!         $itemsAsStrings = implode('', $items);
//!         $itemsAsStrings = implode('|', $items);
//!     }
//! }
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/CodingStyle/Rector/FuncCall/ConsistentImplodeRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_consistent_implode<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ConsistentImplodeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ConsistentImplodeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ConsistentImplodeVisitor<'s> {
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

pub struct ConsistentImplodeRule;

impl Rule for ConsistentImplodeRule {
    fn name(&self) -> &'static str {
        "consistent_implode"
    }

    fn description(&self) -> &'static str {
        "Changes various implode forms to consistent one"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_consistent_implode(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

