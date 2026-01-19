//! Rule: Convert deprecated utf8_decode/utf8_encode to mb_convert_encoding
//!
//! PHP 8.2 deprecated utf8_decode() and utf8_encode() functions.
//! This rule converts them to mb_convert_encoding() equivalents.
//!
//! Transformations:
//! - `utf8_decode($str)` → `mb_convert_encoding($str, 'ISO-8859-1')`
//! - `utf8_encode($str)` → `mb_convert_encoding($str, 'UTF-8', 'ISO-8859-1')`

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for deprecated utf8_decode/utf8_encode calls
pub fn check_utf8_decode_encode<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = Utf8DecodeEncodeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct Utf8DecodeEncodeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for Utf8DecodeEncodeVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(func_call)) = expr {
            if let Some((replacement, message)) = try_transform_utf8_func(func_call, self.source) {
                self.edits.push(Edit::new(expr.span(), replacement, message));
                return false;
            }
        }
        true
    }
}

/// Try to transform utf8_decode/utf8_encode, returning (replacement, message) if successful
fn try_transform_utf8_func(
    func_call: &FunctionCall<'_>,
    source: &str,
) -> Option<(String, &'static str)> {
    // Get function name
    let name = if let Expression::Identifier(ident) = func_call.function {
        let span = ident.span();
        &source[span.start.offset as usize..span.end.offset as usize]
    } else {
        return None;
    };

    // Must have exactly 1 argument
    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
    if args.len() != 1 {
        return None;
    }

    // Get the argument text
    let arg_span = args[0].span();
    let arg_text = &source[arg_span.start.offset as usize..arg_span.end.offset as usize];

    // Check which function and create appropriate replacement
    if name.eq_ignore_ascii_case("utf8_decode") {
        // utf8_decode($str) → mb_convert_encoding($str, 'ISO-8859-1')
        let replacement = format!("mb_convert_encoding({}, 'ISO-8859-1')", arg_text);
        Some((
            replacement,
            "Replace deprecated utf8_decode() with mb_convert_encoding() (PHP 8.2+)",
        ))
    } else if name.eq_ignore_ascii_case("utf8_encode") {
        // utf8_encode($str) → mb_convert_encoding($str, 'UTF-8', 'ISO-8859-1')
        let replacement = format!(
            "mb_convert_encoding({}, 'UTF-8', 'ISO-8859-1')",
            arg_text
        );
        Some((
            replacement,
            "Replace deprecated utf8_encode() with mb_convert_encoding() (PHP 8.2+)",
        ))
    } else {
        None
    }
}

use crate::registry::{Category, PhpVersion, Rule};

pub struct Utf8DecodeEncodeRule;

impl Rule for Utf8DecodeEncodeRule {
    fn name(&self) -> &'static str {
        "utf8_decode_encode"
    }

    fn description(&self) -> &'static str {
        "Convert deprecated utf8_decode/utf8_encode to mb_convert_encoding (PHP 8.2+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_utf8_decode_encode(program, source)
    }

    fn category(&self) -> Category {
        Category::Compatibility
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php82)
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
        check_utf8_decode_encode(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== utf8_decode Tests ====================

    #[test]
    fn test_utf8_decode_simple() {
        let source = "<?php utf8_decode($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php mb_convert_encoding($str, 'ISO-8859-1');"
        );
    }

    #[test]
    fn test_utf8_decode_in_assignment() {
        let source = "<?php $result = utf8_decode($input);";
        assert_eq!(
            transform(source),
            "<?php $result = mb_convert_encoding($input, 'ISO-8859-1');"
        );
    }

    #[test]
    fn test_utf8_decode_in_return() {
        let source = "<?php return utf8_decode($data);";
        assert_eq!(
            transform(source),
            "<?php return mb_convert_encoding($data, 'ISO-8859-1');"
        );
    }

    #[test]
    fn test_utf8_decode_with_expression() {
        let source = "<?php utf8_decode($obj->getData());";
        assert_eq!(
            transform(source),
            "<?php mb_convert_encoding($obj->getData(), 'ISO-8859-1');"
        );
    }

    // ==================== utf8_encode Tests ====================

    #[test]
    fn test_utf8_encode_simple() {
        let source = "<?php utf8_encode($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php mb_convert_encoding($str, 'UTF-8', 'ISO-8859-1');"
        );
    }

    #[test]
    fn test_utf8_encode_in_assignment() {
        let source = "<?php $result = utf8_encode($input);";
        assert_eq!(
            transform(source),
            "<?php $result = mb_convert_encoding($input, 'UTF-8', 'ISO-8859-1');"
        );
    }

    #[test]
    fn test_utf8_encode_in_return() {
        let source = "<?php return utf8_encode($data);";
        assert_eq!(
            transform(source),
            "<?php return mb_convert_encoding($data, 'UTF-8', 'ISO-8859-1');"
        );
    }

    #[test]
    fn test_utf8_encode_with_concat() {
        let source = r#"<?php echo "Result: " . utf8_encode($text);"#;
        assert_eq!(
            transform(source),
            r#"<?php echo "Result: " . mb_convert_encoding($text, 'UTF-8', 'ISO-8859-1');"#
        );
    }

    // ==================== Case Insensitivity ====================

    #[test]
    fn test_utf8_decode_uppercase() {
        let source = "<?php UTF8_DECODE($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_utf8_encode_mixed_case() {
        let source = "<?php Utf8_Encode($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_functions() {
        let source = r#"<?php
$decoded = utf8_decode($input);
$encoded = utf8_encode($other);
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_both_in_expression() {
        let source = "<?php $result = utf8_encode(utf8_decode($data));";
        let edits = check_php(source);
        // Only matches outer call first - inner becomes argument after replacement
        // Running twice would catch the inner one
        assert_eq!(edits.len(), 1);
        assert_eq!(
            transform(source),
            "<?php $result = mb_convert_encoding(utf8_decode($data), 'UTF-8', 'ISO-8859-1');"
        );
    }

    // ==================== Nested Contexts ====================

    #[test]
    fn test_in_array() {
        let source = "<?php $arr = [utf8_decode($a), utf8_encode($b)];";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_in_function_arg() {
        let source = "<?php process(utf8_decode($input));";
        assert_eq!(
            transform(source),
            "<?php process(mb_convert_encoding($input, 'ISO-8859-1'));"
        );
    }

    #[test]
    fn test_in_method() {
        let source = r#"<?php
class Converter {
    public function convert($str) {
        return utf8_decode($str);
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_similar_function() {
        let source = "<?php my_utf8_decode($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php $obj->utf8_decode($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_static_method() {
        let source = "<?php Encoder::utf8_decode($str);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_no_args() {
        // utf8_decode requires an argument
        let source = "<?php utf8_decode();";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_multiple_args() {
        // utf8_decode only takes one argument
        let source = "<?php utf8_decode($a, $b);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn test_with_ternary_arg() {
        let source = "<?php utf8_decode($flag ? $a : $b);";
        assert_eq!(
            transform(source),
            "<?php mb_convert_encoding($flag ? $a : $b, 'ISO-8859-1');"
        );
    }

    #[test]
    fn test_with_array_access() {
        let source = "<?php utf8_encode($data['text']);";
        assert_eq!(
            transform(source),
            "<?php mb_convert_encoding($data['text'], 'UTF-8', 'ISO-8859-1');"
        );
    }

    #[test]
    fn test_with_null_coalesce() {
        let source = "<?php utf8_decode($str ?? 'default');";
        assert_eq!(
            transform(source),
            "<?php mb_convert_encoding($str ?? 'default', 'ISO-8859-1');"
        );
    }
}
