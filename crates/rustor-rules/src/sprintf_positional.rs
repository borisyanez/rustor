//! Rule: Convert simple sprintf() calls to string interpolation
//!
//! Converts simple sprintf patterns to PHP string interpolation for improved
//! readability. Only handles simple %s and %d specifiers without width,
//! precision, or positional arguments.
//!
//! Example:
//! ```php
//! // Before
//! sprintf('%s: %d', $name, $count);
//! sprintf('Hello %s', $name);
//!
//! // After
//! "{$name}: {$count}"
//! "Hello {$name}"
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for simple sprintf patterns
pub fn check_sprintf_positional<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = SprintfVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct SprintfVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for SprintfVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Some(edit) = try_transform_sprintf(expr, self.source) {
            self.edits.push(edit);
            return false;
        }
        true
    }
}

/// Try to transform a sprintf call, returning the Edit if successful
fn try_transform_sprintf(expr: &Expression<'_>, source: &str) -> Option<Edit> {
    // Match function call
    let func_call = match expr {
        Expression::Call(Call::Function(fc)) => fc,
        _ => return None,
    };

    // Check function name is "sprintf"
    let name = match &func_call.function {
        Expression::Identifier(Identifier::Local(local)) => local.value,
        _ => return None,
    };

    if !name.eq_ignore_ascii_case("sprintf") {
        return None;
    }

    // Get arguments
    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

    // Need at least format string
    if args.is_empty() {
        return None;
    }

    // First argument must be a string literal (the format string)
    let format_arg = args[0].value();
    let format_string = extract_string_literal(format_arg)?;

    // Parse the format string for specifiers
    let specifiers = parse_format_specifiers(&format_string)?;

    // Check we have enough arguments
    if specifiers.len() != args.len() - 1 {
        return None;
    }

    // Build the interpolated string
    let mut result = String::with_capacity(format_string.len() * 2);
    result.push('"');

    let mut last_end = 0;
    for (i, spec) in specifiers.iter().enumerate() {
        // Add text before this specifier
        let before = &format_string[last_end..spec.start];
        result.push_str(&escape_for_double_quote(before));

        // Get the corresponding argument expression
        let arg = args[i + 1].value();
        let arg_span = arg.span();
        let arg_code = &source[arg_span.start.offset as usize..arg_span.end.offset as usize];

        // Add the interpolated variable
        // Use {$var} syntax for safety with all expression types
        result.push_str("{");
        result.push_str(arg_code);
        result.push_str("}");

        last_end = spec.end;
    }

    // Add remaining text after last specifier
    let after = &format_string[last_end..];
    result.push_str(&escape_for_double_quote(after));
    result.push('"');

    Some(Edit::new(
        expr.span(),
        result,
        "Convert sprintf() to string interpolation",
    ))
}

/// Extract a string literal value, returning None if not a simple string
fn extract_string_literal(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::Literal(Literal::String(string_lit)) => {
            // The `value` field contains the string content without quotes
            // The `raw` field contains the original including quotes
            // The `kind` field tells us if it's single or double quoted

            // If value is available, use it directly (it's already unquoted)
            if let Some(value) = string_lit.value {
                return Some(value.to_string());
            }

            // Fallback: parse from raw if value is None
            let raw = string_lit.raw;
            if raw.starts_with('\'') && raw.ends_with('\'') {
                let inner = &raw[1..raw.len() - 1];
                Some(inner.replace("\\'", "'").replace("\\\\", "\\"))
            } else if raw.starts_with('"') && raw.ends_with('"') {
                let inner = &raw[1..raw.len() - 1];
                Some(unescape_double_quoted(inner))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Unescape a double-quoted string content
fn unescape_double_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                match next {
                    'n' => {
                        result.push('\n');
                        chars.next();
                    }
                    'r' => {
                        result.push('\r');
                        chars.next();
                    }
                    't' => {
                        result.push('\t');
                        chars.next();
                    }
                    '\\' => {
                        result.push('\\');
                        chars.next();
                    }
                    '"' => {
                        result.push('"');
                        chars.next();
                    }
                    '$' => {
                        result.push('$');
                        chars.next();
                    }
                    _ => {
                        result.push('\\');
                    }
                }
            } else {
                result.push('\\');
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Escape string content for a double-quoted string
fn escape_for_double_quote(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '$' => result.push_str("\\$"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

/// A format specifier in the format string
struct FormatSpecifier {
    start: usize,
    end: usize,
}

/// Parse format specifiers from a format string.
/// Returns None if the format string contains unsupported specifiers.
fn parse_format_specifiers(format: &str) -> Option<Vec<FormatSpecifier>> {
    let mut specifiers = Vec::new();
    let mut i = 0;
    let bytes = format.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 1 >= bytes.len() {
                return None; // Invalid: % at end
            }

            let spec_start = i;
            i += 1;

            match bytes[i] {
                // %% is an escaped percent sign, not a specifier
                b'%' => {
                    i += 1;
                    continue;
                }
                // Simple specifiers we support
                b's' | b'd' => {
                    i += 1;
                    specifiers.push(FormatSpecifier {
                        start: spec_start,
                        end: i,
                    });
                }
                // Any other specifier (width, precision, positional, etc.) - bail out
                _ => return None,
            }
        } else {
            i += 1;
        }
    }

    Some(specifiers)
}

use crate::registry::{Category, Rule};

pub struct SprintfPositionalRule;

impl Rule for SprintfPositionalRule {
    fn name(&self) -> &'static str {
        "sprintf_positional"
    }

    fn description(&self) -> &'static str {
        "Convert simple sprintf() to string interpolation"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_sprintf_positional(program, source)
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
        check_sprintf_positional(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_string_interpolation() {
        let source = "<?php sprintf('%s', $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php \"{$name}\";");
    }

    #[test]
    fn test_string_with_text() {
        let source = "<?php sprintf('Hello %s', $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php \"Hello {$name}\";");
    }

    #[test]
    fn test_multiple_args() {
        let source = "<?php sprintf('%s: %d', $name, $count);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php \"{$name}: {$count}\";");
    }

    #[test]
    fn test_double_quoted_format() {
        let source = r#"<?php sprintf("%s", $name);"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php \"{$name}\";");
    }

    #[test]
    fn test_complex_expression() {
        let source = "<?php sprintf('%s', $obj->getName());";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php \"{$obj->getName()}\";");
    }

    #[test]
    fn test_array_access() {
        let source = "<?php sprintf('%s', $arr['key']);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php \"{$arr['key']}\";");
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_width_specifier() {
        // %10s has width - not supported
        let source = "<?php sprintf('%10s', $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_precision_specifier() {
        // %.2f has precision - not supported
        let source = "<?php sprintf('%.2f', $value);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_positional_specifier() {
        // %1$s has positional - not supported
        let source = "<?php sprintf('%1$s', $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_float_specifier() {
        // %f is not simple enough
        let source = "<?php sprintf('%f', $value);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_mismatched_args() {
        // 2 specifiers but only 1 arg
        let source = "<?php sprintf('%s %s', $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_variable_format() {
        // Format string is a variable, not a literal
        let source = "<?php sprintf($format, $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_escaped_percent() {
        // %% should be escaped and not count as a specifier
        let source = "<?php sprintf('100%% of %s', $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        // Note: %% becomes % in the result
        assert_eq!(transform(source), "<?php \"100%% of {$name}\";");
    }

    #[test]
    fn test_empty_format_args() {
        // No specifiers - technically valid but no point
        let source = "<?php sprintf('Hello');";
        let edits = check_php(source);
        // This should be converted to just "Hello"
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php \"Hello\";");
    }

    #[test]
    fn test_special_chars_in_format() {
        let source = r#"<?php sprintf('Name: %s\n', $name);"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Nested Context Tests ====================

    #[test]
    fn test_in_assignment() {
        let source = "<?php $msg = sprintf('%s: %d', $name, $count);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php $msg = \"{$name}: {$count}\";");
    }

    #[test]
    fn test_in_function_call() {
        let source = "<?php echo sprintf('Hello %s', $name);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(transform(source), "<?php echo \"Hello {$name}\";");
    }

    #[test]
    fn test_multiple_sprintf_calls() {
        let source = "<?php $a = sprintf('%s', $x); $b = sprintf('%d', $y);";
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
    }
}
