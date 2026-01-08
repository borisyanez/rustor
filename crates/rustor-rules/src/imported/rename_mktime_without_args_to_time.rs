//! Rule: Renames mktime() without arguments to time()
//!
//! Example:
//! ```php
//! // Before
//! class SomeClass
//! {
//!     public function run()
//!     {
//!         $time = mktime(1, 2, 3);
//!         $nextTime = mktime();
//!     }
//! }
//!
//! // After
//! class SomeClass
//! {
//!     public function run()
//!     {
//!         $time = mktime(1, 2, 3);
//!         $nextTime = time();
//!     }
//! }
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php70/Rector/FuncCall/RenameMktimeWithoutArgsToTimeRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_rename_mktime_without_args_to_time<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RenameMktimeWithoutArgsToTimeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RenameMktimeWithoutArgsToTimeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RenameMktimeWithoutArgsToTimeVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("mktime") {
                self.edits.push(Edit::new(
                    call.function.span(),
                    "time",
                    "Renames mktime() without arguments to time()",
                ));
            }
        }
        true
    }
}

pub struct RenameMktimeWithoutArgsToTimeRule;

impl Rule for RenameMktimeWithoutArgsToTimeRule {
    fn name(&self) -> &'static str {
        "rename_mktime_without_args_to_time"
    }

    fn description(&self) -> &'static str {
        "Renames mktime() without arguments to time()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_rename_mktime_without_args_to_time(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php70)
    }
}

