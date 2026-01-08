//! Rule: Change filter_var() with slash escaping to addslashes()
//!
//! Example:
//! ```php
//! // Before
//! $var= "Satya's here!";
//! filter_var($var, FILTER_SANITIZE_MAGIC_QUOTES);
//!
//! // After
//! $var= "Satya's here!";
//! addslashes($var);
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php74/Rector/FuncCall/FilterVarToAddSlashesRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_filter_var_to_add_slashes<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = FilterVarToAddSlashesVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct FilterVarToAddSlashesVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for FilterVarToAddSlashesVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("filter_var") {
                self.edits.push(Edit::new(
                    call.function.span(),
                    "addslashes",
                    "Change filter_var() with slash escaping to addslashes()",
                ));
            }
        }
        true
    }
}

pub struct FilterVarToAddSlashesRule;

impl Rule for FilterVarToAddSlashesRule {
    fn name(&self) -> &'static str {
        "filter_var_to_add_slashes"
    }

    fn description(&self) -> &'static str {
        "Change filter_var() with slash escaping to addslashes()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_filter_var_to_add_slashes(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php74)
    }
}

