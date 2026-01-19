//! Rule: Simplify array_merge of array literals to single array
//!
//! When all arguments to array_merge are array literals, they can be combined into
//! a single array literal.
//!
//! Transformation:
//! - `array_merge([$a], [$b, $c])` → `[$a, $b, $c]`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for array_merge with array literals
pub fn check_array_merge_simple<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = ArrayMergeSimpleVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ArrayMergeSimpleVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for ArrayMergeSimpleVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some(edit) = try_simplify_array_merge(func_call, self.source) {
                self.edits.push(edit);
                return false;
            }
        }
        true
    }
}

/// Try to simplify array_merge of array literals
fn try_simplify_array_merge(func_call: &FunctionCall<'_>, source: &str) -> Option<Edit> {
    // Check function name is "array_merge"
    let func_name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    if !func_name.eq_ignore_ascii_case("array_merge") {
        return None;
    }

    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Must have at least 2 arguments for this to make sense
    if args.len() < 2 {
        return None;
    }

    // Collect all elements from all array literals
    let mut all_elements: Vec<String> = Vec::new();

    for arg in &args {
        let arg_value = arg.value();

        // Must be an array literal
        let array = if let Expression::Array(arr) = arg_value {
            arr
        } else {
            return None;
        };

        // Collect each element's text
        for element in array.elements.iter() {
            let elem_span = element.span();
            let elem_text = &source[elem_span.start.offset as usize..elem_span.end.offset as usize];
            all_elements.push(elem_text.to_string());
        }
    }

    // Create the merged array
    let func_span = func_call.span();
    let replacement = format!("[{}]", all_elements.join(", "));

    Some(Edit::new(
        func_span,
        replacement,
        "Simplify array_merge to single array",
    ))
}

use crate::registry::{Category, Rule};

pub struct ArrayMergeSimpleRule;

impl Rule for ArrayMergeSimpleRule {
    fn name(&self) -> &'static str {
        "array_merge_simple"
    }

    fn description(&self) -> &'static str {
        "Simplify array_merge of array literals to single array"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_array_merge_simple(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
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
        check_array_merge_simple(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Patterns ====================

    #[test]
    fn test_basic() {
        let source = "<?php array_merge([$a], [$b]);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php [$a, $b];");
    }

    #[test]
    fn test_three_arrays() {
        let source = "<?php array_merge([$a], [$b], [$c]);";
        assert_eq!(transform(source), "<?php [$a, $b, $c];");
    }

    #[test]
    fn test_with_values() {
        let source = "<?php array_merge([1, 2], [3, 4]);";
        assert_eq!(transform(source), "<?php [1, 2, 3, 4];");
    }

    #[test]
    fn test_with_strings() {
        let source = r#"<?php array_merge(['a'], ['b', 'c']);"#;
        assert_eq!(transform(source), r#"<?php ['a', 'b', 'c'];"#);
    }

    // ==================== In Context ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $arr = array_merge([$x], [$y]);";
        assert_eq!(transform(source), "<?php $arr = [$x, $y];");
    }

    #[test]
    fn test_in_return() {
        let source = "<?php return array_merge([1], [2, 3]);";
        assert_eq!(transform(source), "<?php return [1, 2, 3];");
    }

    // ==================== With Keys ====================

    #[test]
    fn test_with_keys() {
        let source = r#"<?php array_merge(['a' => 1], ['b' => 2]);"#;
        assert_eq!(transform(source), r#"<?php ['a' => 1, 'b' => 2];"#);
    }

    // ==================== Multiple ====================

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = array_merge([1], [2]);
$b = array_merge([3], [4]);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_single_arg() {
        let source = "<?php array_merge([$a]);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_variable_arg() {
        // One arg is a variable, not an array literal
        let source = "<?php array_merge([$a], $arr);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_function_call_arg() {
        let source = "<?php array_merge([$a], getArray());";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_other_function() {
        let source = "<?php array_combine([$a], [$b]);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
