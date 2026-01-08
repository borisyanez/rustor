//! Rule: Change restore_include_path() to ini_restore(
//!
//! Example:
//! ```php
//! // Before
//! restore_include_path();
//!
//! // After
//! ini_restore('include_path');
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php74/Rector/FuncCall/RestoreIncludePathToIniRestoreRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_restore_include_path_to_ini_restore<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RestoreIncludePathToIniRestoreVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RestoreIncludePathToIniRestoreVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RestoreIncludePathToIniRestoreVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("restore_include_path") {
                self.edits.push(Edit::new(
                    call.function.span(),
                    "ini_restore",
                    "Change restore_include_path() to ini_restore(",
                ));
            }
        }
        true
    }
}

pub struct RestoreIncludePathToIniRestoreRule;

impl Rule for RestoreIncludePathToIniRestoreRule {
    fn name(&self) -> &'static str {
        "restore_include_path_to_ini_restore"
    }

    fn description(&self) -> &'static str {
        "Change restore_include_path() to ini_restore("
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_restore_include_path_to_ini_restore(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php74)
    }
}

