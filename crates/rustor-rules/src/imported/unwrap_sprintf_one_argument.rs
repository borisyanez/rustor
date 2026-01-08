//! Rule: Unwrap `sprintf()` with one argument
//!
//! Example:
//! ```php
//! // Before
//! echo sprintf('value');
//!
//! // After
//! echo 'value';
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/CodeQuality/Rector/FuncCall/UnwrapSprintfOneArgumentRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_unwrap_sprintf_one_argument<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = UnwrapSprintfOneArgumentVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct UnwrapSprintfOneArgumentVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for UnwrapSprintfOneArgumentVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("sprintf") {
                // Must have exactly 1 argument to unwrap
                let args: Vec<_> = call.argument_list.arguments.iter().collect();
                if args.len() == 1 {
                    let arg_str = &self.source[args[0].span().start.offset as usize..args[0].span().end.offset as usize];

                    self.edits.push(Edit::new(
                        expr.span(),
                        arg_str,
                        "Remove unnecessary sprintf() wrapper",
                    ));
                }
            }
        }
        true
    }
}

pub struct UnwrapSprintfOneArgumentRule;

impl Rule for UnwrapSprintfOneArgumentRule {
    fn name(&self) -> &'static str {
        "unwrap_sprintf_one_argument"
    }

    fn description(&self) -> &'static str {
        "Unwrap `sprintf()` with one argument"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_unwrap_sprintf_one_argument(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

