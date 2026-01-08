//! Rule: Change array_key_exists() on property to property_exists()
//!
//! Example:
//! ```php
//! // Before
//! class SomeClass
//! {
//!      public $value;
//! }
//! $someClass = new SomeClass;
//! 
//! array_key_exists('value', $someClass);
//!
//! // After
//! class SomeClass
//! {
//!      public $value;
//! }
//! $someClass = new SomeClass;
//! 
//! property_exists($someClass, 'value');
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php74/Rector/FuncCall/ArrayKeyExistsOnPropertyRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_array_key_exists_on_property<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ArrayKeyExistsOnPropertyVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ArrayKeyExistsOnPropertyVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ArrayKeyExistsOnPropertyVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("array_key_exists") {
                self.edits.push(Edit::new(
                    call.function.span(),
                    "property_exists",
                    "Change array_key_exists() on property to property_exists()",
                ));
            }
        }
        true
    }
}

pub struct ArrayKeyExistsOnPropertyRule;

impl Rule for ArrayKeyExistsOnPropertyRule {
    fn name(&self) -> &'static str {
        "array_key_exists_on_property"
    }

    fn description(&self) -> &'static str {
        "Change array_key_exists() on property to property_exists()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_array_key_exists_on_property(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php74)
    }
}

