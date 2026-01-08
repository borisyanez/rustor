//! Rule: Change helper functions to str_starts_with()
//!
//! Converts patterns like:
//! - `substr($haystack, 0, strlen($needle)) === $needle`
//! - `strncmp($haystack, $needle, strlen($needle)) === 0`
//! - `strpos($haystack, $needle) === 0`
//!
//! To: `str_starts_with($haystack, $needle)`
//!
//! Imported from Rector: StrStartsWithRector.php

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_str_starts_with<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = StrStartsWithVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StrStartsWithVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> StrStartsWithVisitor<'s> {
    /// Check if expression is substr($haystack, 0, strlen($needle))
    fn match_substr_pattern(&self, expr: &Expression<'_>) -> Option<(String, String)> {
        if let Expression::Call(Call::Function(call)) = expr {
            let name = self.get_func_name(call)?;
            if !name.eq_ignore_ascii_case("substr") {
                return None;
            }

            let args: Vec<_> = call.argument_list.arguments.iter().collect();
            if args.len() < 3 {
                return None;
            }

            // Check second arg is 0
            let second_arg = self.get_expr_text(args[1]);
            if second_arg.trim() != "0" {
                return None;
            }

            // Check third arg is strlen($needle)
            let third_arg = args[2];
            if let Argument::Positional(pos) = third_arg {
                if let Expression::Call(Call::Function(strlen_call)) = &pos.value {
                    let strlen_name = self.get_func_name(strlen_call)?;
                    if strlen_name.eq_ignore_ascii_case("strlen") {
                        let haystack = self.get_expr_text(args[0]);
                        // Get first argument from strlen call
                        if let Some(first_arg) = strlen_call.argument_list.arguments.iter().next() {
                            let needle = self.get_expr_text(first_arg);
                            return Some((haystack, needle));
                        }
                    }
                }
            }
        }
        None
    }

    /// Check if expression is strpos($haystack, $needle) with comparison to 0
    fn match_strpos_pattern(&self, expr: &Expression<'_>) -> Option<(String, String)> {
        if let Expression::Call(Call::Function(call)) = expr {
            let name = self.get_func_name(call)?;
            if !name.eq_ignore_ascii_case("strpos") {
                return None;
            }

            let args: Vec<_> = call.argument_list.arguments.iter().collect();
            if args.len() < 2 {
                return None;
            }

            let haystack = self.get_expr_text(args[0]);
            let needle = self.get_expr_text(args[1]);
            return Some((haystack, needle));
        }
        None
    }

    fn get_func_name(&self, call: &FunctionCall<'_>) -> Option<String> {
        let span = call.function.span();
        Some(self.source[span.start.offset as usize..span.end.offset as usize].to_string())
    }

    fn get_expr_text(&self, arg: &Argument<'_>) -> String {
        let span = arg.span();
        self.source[span.start.offset as usize..span.end.offset as usize].to_string()
    }

}

impl<'a, 's> Visitor<'a> for StrStartsWithVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Match: substr($h, 0, strlen($n)) === $n
        // Match: strpos($h, $n) === 0
        if let Expression::Binary(binary) = expr {
            let op_span = binary.operator.span();
            let op = &self.source[op_span.start.offset as usize..op_span.end.offset as usize];

            let is_identical = op == "===" || op == "==";
            let is_not_identical = op == "!==" || op == "!=";

            if !is_identical && !is_not_identical {
                return true;
            }

            // Pattern 1: substr($haystack, 0, strlen($needle)) === $needle
            if let Some((haystack, needle)) = self.match_substr_pattern(&binary.lhs) {
                let rhs = &self.source[binary.rhs.span().start.offset as usize..binary.rhs.span().end.offset as usize];

                // Check that RHS matches the needle (or is the same variable)
                if rhs.trim() == needle.trim() || self.vars_likely_equal(rhs, &needle) {
                    let replacement = if is_not_identical {
                        format!("!str_starts_with({}, {})", haystack, needle)
                    } else {
                        format!("str_starts_with({}, {})", haystack, needle)
                    };

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Use str_starts_with() (PHP 8.0+)",
                    ));
                    return true;
                }
            }

            // Pattern 2: strpos($haystack, $needle) === 0
            if let Some((haystack, needle)) = self.match_strpos_pattern(&binary.lhs) {
                let rhs = &self.source[binary.rhs.span().start.offset as usize..binary.rhs.span().end.offset as usize];

                if rhs.trim() == "0" {
                    let replacement = if is_not_identical {
                        format!("!str_starts_with({}, {})", haystack, needle)
                    } else {
                        format!("str_starts_with({}, {})", haystack, needle)
                    };

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Use str_starts_with() (PHP 8.0+)",
                    ));
                    return true;
                }
            }

            // Also check reversed: $needle === substr(...)
            if let Some((haystack, needle)) = self.match_substr_pattern(&binary.rhs) {
                let lhs = &self.source[binary.lhs.span().start.offset as usize..binary.lhs.span().end.offset as usize];

                if lhs.trim() == needle.trim() || self.vars_likely_equal(lhs, &needle) {
                    let replacement = if is_not_identical {
                        format!("!str_starts_with({}, {})", haystack, needle)
                    } else {
                        format!("str_starts_with({}, {})", haystack, needle)
                    };

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Use str_starts_with() (PHP 8.0+)",
                    ));
                }
            }
        }
        true
    }
}

impl<'s> StrStartsWithVisitor<'s> {
    fn vars_likely_equal(&self, a: &str, b: &str) -> bool {
        // Simple heuristic: if both start with $ and have same name
        let a = a.trim();
        let b = b.trim();
        a == b
    }
}

pub struct StrStartsWithRule;

impl Rule for StrStartsWithRule {
    fn name(&self) -> &'static str {
        "str_starts_with"
    }

    fn description(&self) -> &'static str {
        "Change helper functions to str_starts_with()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_str_starts_with(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;

    fn parse_and_check(code: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, code);
        check_str_starts_with(&program, code)
    }

    #[test]
    fn test_substr_strlen_pattern() {
        let code = r#"<?php
$isMatch = substr($haystack, 0, strlen($needle)) === $needle;
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("str_starts_with"));
    }

    #[test]
    fn test_strpos_zero_pattern() {
        let code = r#"<?php
$isMatch = strpos($haystack, $needle) === 0;
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.contains("str_starts_with"));
    }

    #[test]
    fn test_negated_pattern() {
        let code = r#"<?php
$isNotMatch = strpos($haystack, $needle) !== 0;
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert!(edits[0].replacement.starts_with("!str_starts_with"));
    }

    #[test]
    fn test_no_match_different_offset() {
        let code = r#"<?php
$x = substr($haystack, 1, strlen($needle)) === $needle;
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 0); // offset is 1, not 0
    }
}
