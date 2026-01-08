//! Rule: Change hebrevc($str) to nl2br(hebrev($str))
//!
//! Example:
//! ```php
//! // Before
//! hebrevc($str);
//!
//! // After
//! nl2br(hebrev($str));
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php74/Rector/FuncCall/HebrevcToNl2brHebrevRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_hebrevc_to_nl_2_br_hebrev<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = HebrevcToNl2BrHebrevVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct HebrevcToNl2BrHebrevVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for HebrevcToNl2BrHebrevVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("hebrevc") {
                self.edits.push(Edit::new(
                    call.function.span(),
                    "hebrev",
                    "Change hebrevc($str) to nl2br(hebrev($str))",
                ));
            }
        }
        true
    }
}

pub struct HebrevcToNl2BrHebrevRule;

impl Rule for HebrevcToNl2BrHebrevRule {
    fn name(&self) -> &'static str {
        "hebrevc_to_nl_2_br_hebrev"
    }

    fn description(&self) -> &'static str {
        "Change hebrevc($str) to nl2br(hebrev($str))"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_hebrevc_to_nl_2_br_hebrev(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php74)
    }
}

