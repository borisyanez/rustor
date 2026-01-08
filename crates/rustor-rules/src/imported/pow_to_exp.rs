//! Rule: Changes `pow(val, val2)` to `**` (exp) parameter
//!
//! Example:
//! ```php
//! // Before
//! pow(1, 2);
//!
//! // After
//! 1**2;
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php56/Rector/FuncCall/PowToExpRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_pow_to_exp<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = PowToExpVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct PowToExpVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for PowToExpVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("pow") {
                // Need exactly 2 arguments
                let args: Vec<_> = call.argument_list.arguments.iter().collect();
                if args.len() == 2 {
                    let left = &self.source[args[0].span().start.offset as usize..args[0].span().end.offset as usize];
                    let right = &self.source[args[1].span().start.offset as usize..args[1].span().end.offset as usize];
                    let replacement = format!("{} ** {}", left, right);

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Replace pow() with ** operator",
                    ));
                }
            }
        }
        true
    }
}

pub struct PowToExpRule;

impl Rule for PowToExpRule {
    fn name(&self) -> &'static str {
        "pow_to_exp"
    }

    fn description(&self) -> &'static str {
        "Changes `pow(val, val2)` to `**` (exp) parameter"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_pow_to_exp(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php56)
    }
}

