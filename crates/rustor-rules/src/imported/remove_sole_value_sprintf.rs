//! Rule: Remove `sprintf()` wrapper if not needed
//!
//! Example:
//! ```php
//! // Before
//! class SomeClass
//! {
//!     public function run()
//!     {
//!         $welcome = 'hello';
//!         $value = sprintf('%s', $welcome);
//!     }
//! }
//!
//! // After
//! class SomeClass
//! {
//!     public function run()
//!     {
//!         $welcome = 'hello';
//!         $value = $welcome;
//!     }
//! }
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/CodeQuality/Rector/FuncCall/RemoveSoleValueSprintfRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_remove_sole_value_sprintf<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveSoleValueSprintfVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveSoleValueSprintfVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RemoveSoleValueSprintfVisitor<'s> {
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

pub struct RemoveSoleValueSprintfRule;

impl Rule for RemoveSoleValueSprintfRule {
    fn name(&self) -> &'static str {
        "remove_sole_value_sprintf"
    }

    fn description(&self) -> &'static str {
        "Remove `sprintf()` wrapper if not needed"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_sole_value_sprintf(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

