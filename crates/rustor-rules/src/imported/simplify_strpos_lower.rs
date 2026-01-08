//! Rule: Simplify strpos(strtolower(),
//!
//! Example:
//! ```php
//! // Before
//! strpos(strtolower($var), "...")
//!
//! // After
//! stripos($var, "...")
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/CodeQuality/Rector/FuncCall/SimplifyStrposLowerRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_simplify_strpos_lower<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SimplifyStrposLowerVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SimplifyStrposLowerVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SimplifyStrposLowerVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("strpos") {
                self.edits.push(Edit::new(
                    call.function.span(),
                    "stripos",
                    "Simplify strpos(strtolower(),",
                ));
            }
        }
        true
    }
}

pub struct SimplifyStrposLowerRule;

impl Rule for SimplifyStrposLowerRule {
    fn name(&self) -> &'static str {
        "simplify_strpos_lower"
    }

    fn description(&self) -> &'static str {
        "Simplify strpos(strtolower(),"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_simplify_strpos_lower(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

