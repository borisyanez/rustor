//! Rule: Replace calls to `get_class()` and `get_parent_class()` without arguments with `self::class` and `parent::class`
//!
//! Example:
//! ```php
//! // Before

//!
//! // After

//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php83/Rector/FuncCall/RemoveGetClassGetParentClassNoArgsRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_remove_get_class_get_parent_class_no_args<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RemoveGetClassGetParentClassNoArgsVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RemoveGetClassGetParentClassNoArgsVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RemoveGetClassGetParentClassNoArgsVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("get_class") {
                // Get the argument
                if let Some(arg) = call.argument_list.arguments.first() {
                    let arg_str = &self.source[arg.span().start.offset as usize..arg.span().end.offset as usize];
                    let replacement = format!("{}::class", arg_str);

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Replace get_class() with ::class constant",
                    ));
                }
            }
        }
        true
    }
}

pub struct RemoveGetClassGetParentClassNoArgsRule;

impl Rule for RemoveGetClassGetParentClassNoArgsRule {
    fn name(&self) -> &'static str {
        "remove_get_class_get_parent_class_no_args"
    }

    fn description(&self) -> &'static str {
        "Replace calls to `get_class()` and `get_parent_class()` without arguments with `self::class` and `parent::class`"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_remove_get_class_get_parent_class_no_args(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php83)
    }
}

