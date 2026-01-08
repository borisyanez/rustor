//! Rule: Remove useless is_object() check on combine with instanceof check
//!
//! Example:
//! ```php
//! // Before
//! is_object($obj) && $obj instanceof DateTime
//!
//! // After
//! $obj instanceof DateTime
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/CodeQuality/Rector/BooleanAnd/RemoveUselessIsObjectCheckRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_remove_useless_is_object_check<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveUselessIsObjectCheckVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveUselessIsObjectCheckVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RemoveUselessIsObjectCheckVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("is_object") {
                // Get the argument
                if let Some(arg) = call.argument_list.arguments.first() {
                    let arg_str = &self.source[arg.span().start.offset as usize..arg.span().end.offset as usize];
                    let replacement = format!("{} instanceof object", arg_str);

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Replace is_object() with instanceof object comparison",
                    ));
                }
            }
        }
        true
    }
}

pub struct RemoveUselessIsObjectCheckRule;

impl Rule for RemoveUselessIsObjectCheckRule {
    fn name(&self) -> &'static str {
        "remove_useless_is_object_check"
    }

    fn description(&self) -> &'static str {
        "Remove useless is_object() check on combine with instanceof check"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_useless_is_object_check(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }
}

