//! Rule: The /e modifier is no longer supported, use preg_replace_callback instead
//!
//! Example:
//! ```php
//! // Before
//! class SomeClass
//! {
//!     public function run()
//!     {
//!         $comment = preg_replace('~\b(\w)(\w+)~e', '"$1".strtolower("$2")', $comment);
//!     }
//! }
//!
//! // After
//! class SomeClass
//! {
//!     public function run()
//!     {
//!         $comment = preg_replace_callback('~\b(\w)(\w+)~', function ($matches) {
//!               return($matches[1].strtolower($matches[2]));
//!         }, $comment);
//!     }
//! }
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php55/Rector/FuncCall/PregReplaceEModifierRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_preg_replace_e_modifier<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = PregReplaceEModifierVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct PregReplaceEModifierVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for PregReplaceEModifierVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("preg_replace") {
                self.edits.push(Edit::new(
                    call.function.span(),
                    "preg_replace_callback",
                    "The /e modifier is no longer supported, use preg_replace_callback instead",
                ));
            }
        }
        true
    }
}

pub struct PregReplaceEModifierRule;

impl Rule for PregReplaceEModifierRule {
    fn name(&self) -> &'static str {
        "preg_replace_e_modifier"
    }

    fn description(&self) -> &'static str {
        "The /e modifier is no longer supported, use preg_replace_callback instead"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_preg_replace_e_modifier(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php55)
    }
}

