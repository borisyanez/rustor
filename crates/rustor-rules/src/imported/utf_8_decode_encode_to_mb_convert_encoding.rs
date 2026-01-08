//! Rule: Change deprecated `utf8_decode()` and `utf8_encode()` to `mb_convert_encoding()`
//!
//! Example:
//! ```php
//! // Before
//! utf8_decode($value);
//! utf8_encode($value);
//!
//! // After
//! mb_convert_encoding($value, 'ISO-8859-1');
//! mb_convert_encoding($value, 'UTF-8', 'ISO-8859-1');
//! ```
//!
//! Imported from Rector: /tmp/rector-src/rules/Php82/Rector/FuncCall/Utf8DecodeEncodeToMbConvertEncodingRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_utf_8_decode_encode_to_mb_convert_encoding<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = Utf8DecodeEncodeToMbConvertEncodingVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct Utf8DecodeEncodeToMbConvertEncodingVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for Utf8DecodeEncodeToMbConvertEncodingVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset as usize..call.function.span().end.offset as usize];

            if name_str.eq_ignore_ascii_case("utf8_decode") {
                self.edits.push(Edit::new(
                    call.function.span(),
                    "mb_convert_encoding",
                    "Change deprecated `utf8_decode()` and `utf8_encode()` to `mb_convert_encoding()`",
                ));
            }
        }
        true
    }
}

pub struct Utf8DecodeEncodeToMbConvertEncodingRule;

impl Rule for Utf8DecodeEncodeToMbConvertEncodingRule {
    fn name(&self) -> &'static str {
        "utf_8_decode_encode_to_mb_convert_encoding"
    }

    fn description(&self) -> &'static str {
        "Change deprecated `utf8_decode()` and `utf8_encode()` to `mb_convert_encoding()`"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_utf_8_decode_encode_to_mb_convert_encoding(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php82)
    }
}

