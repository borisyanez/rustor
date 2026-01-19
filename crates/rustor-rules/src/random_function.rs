//! Rule: Replace deprecated random functions with modern alternatives
//!
//! Since PHP 7.0, the old random functions are deprecated in favor of CSPRNG functions.
//!
//! Transformations:
//! - `getrandmax()` → `mt_getrandmax()`
//! - `srand()` → `mt_srand()`
//! - `srand($seed)` → `mt_srand($seed)`
//! - `rand()` → `random_int(0, mt_getrandmax())`
//! - `rand($min, $max)` → `random_int($min, $max)`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for deprecated random functions
pub fn check_random_function<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RandomFunctionVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RandomFunctionVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RandomFunctionVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_replace_random_function(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to replace deprecated random functions
fn try_replace_random_function(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    let func_name_lower = func_name.to_ascii_lowercase();
    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
    let func_span = func_call.span();

    match func_name_lower.as_str() {
        "getrandmax" => {
            if args.is_empty() {
                return Some(Edit::new(
                    func_span,
                    "mt_getrandmax()".to_string(),
                    "Replace getrandmax() with mt_getrandmax()",
                ));
            }
        }
        "srand" => {
            if args.is_empty() {
                return Some(Edit::new(
                    func_span,
                    "mt_srand()".to_string(),
                    "Replace srand() with mt_srand()",
                ));
            } else if args.len() == 1 {
                let arg_span = args[0].value().span();
                let arg_text = &source[arg_span.start.offset as usize..arg_span.end.offset as usize];
                return Some(Edit::new(
                    func_span,
                    format!("mt_srand({})", arg_text),
                    "Replace srand() with mt_srand()",
                ));
            }
        }
        "rand" => {
            if args.is_empty() {
                return Some(Edit::new(
                    func_span,
                    "random_int(0, mt_getrandmax())".to_string(),
                    "Replace rand() with random_int()",
                ));
            } else if args.len() == 2 {
                let min_span = args[0].value().span();
                let max_span = args[1].value().span();
                let min_text = &source[min_span.start.offset as usize..min_span.end.offset as usize];
                let max_text = &source[max_span.start.offset as usize..max_span.end.offset as usize];
                return Some(Edit::new(
                    func_span,
                    format!("random_int({}, {})", min_text, max_text),
                    "Replace rand() with random_int()",
                ));
            }
        }
        _ => {}
    }

    None
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct RandomFunctionRule;

impl Rule for RandomFunctionRule {
    fn name(&self) -> &'static str {
        "random_function"
    }

    fn description(&self) -> &'static str {
        "Replace deprecated random functions with modern alternatives"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_random_function(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php70)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_random_function(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== getrandmax ====================

    #[test]
    fn test_getrandmax() {
        let source = "<?php getrandmax();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php mt_getrandmax();");
    }

    // ==================== srand ====================

    #[test]
    fn test_srand_no_args() {
        let source = "<?php srand();";
        assert_eq!(transform(source), "<?php mt_srand();");
    }

    #[test]
    fn test_srand_with_seed() {
        let source = "<?php srand(42);";
        assert_eq!(transform(source), "<?php mt_srand(42);");
    }

    #[test]
    fn test_srand_with_variable() {
        let source = "<?php srand($seed);";
        assert_eq!(transform(source), "<?php mt_srand($seed);");
    }

    // ==================== rand ====================

    #[test]
    fn test_rand_no_args() {
        let source = "<?php rand();";
        assert_eq!(transform(source), "<?php random_int(0, mt_getrandmax());");
    }

    #[test]
    fn test_rand_with_range() {
        let source = "<?php rand(1, 100);";
        assert_eq!(transform(source), "<?php random_int(1, 100);");
    }

    #[test]
    fn test_rand_with_variables() {
        let source = "<?php rand($min, $max);";
        assert_eq!(transform(source), "<?php random_int($min, $max);");
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $num = rand(1, 10);";
        assert_eq!(transform(source), "<?php $num = random_int(1, 10);");
    }

    #[test]
    fn test_in_condition() {
        let source = "<?php if (rand(0, 1)) {}";
        assert_eq!(transform(source), "<?php if (random_int(0, 1)) {}");
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
srand(123);
$max = getrandmax();
$num = rand(0, $max);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 3);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_mt_rand() {
        // mt_rand is the replacement, don't change it
        let source = "<?php mt_rand(1, 100);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_random_int() {
        let source = "<?php random_int(1, 100);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_rand_one_arg() {
        // rand() with one arg is invalid, don't transform
        let source = "<?php rand(100);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
