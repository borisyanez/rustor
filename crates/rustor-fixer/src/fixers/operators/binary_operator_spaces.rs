//! Configure spacing around binary operators

use rustor_core::Edit;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures single space around binary operators
pub struct BinaryOperatorSpacesFixer;

impl Fixer for BinaryOperatorSpacesFixer {
    fn name(&self) -> &'static str {
        "binary_operator_spaces"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "binary_operator_spaces"
    }

    fn description(&self) -> &'static str {
        "Ensure single space around binary operators"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();
        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let byte_pos: usize = chars[..i].iter().map(|c| c.len_utf8()).sum();

            // Skip strings and comments
            if is_in_string_or_comment(&source[..byte_pos]) {
                i += 1;
                continue;
            }

            let curr = chars[i];
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = if i + 1 < len { Some(chars[i + 1]) } else { None };
            let next2 = if i + 2 < len { Some(chars[i + 2]) } else { None };

            // Handle 3-character operators first: ===, !==, **=, <<=, >>=, <=>
            if i + 2 < len {
                let three = format!("{}{}{}", curr, next.unwrap(), next2.unwrap());
                if matches!(three.as_str(), "===" | "!==" | "**=" | "<<=" | ">>=" | "<=>") {
                    if let Some(edit) = check_operator_spacing(&chars, i, 3, &three) {
                        edits.push(edit);
                    }
                    i += 3;
                    continue;
                }
            }

            // Handle 2-character operators: ==, !=, <=, >=, <>, +=, -=, *=, /=, .=, %=, &=, |=, ^=, ??=, **, &&, ||, =>, <<, >>
            if i + 1 < len {
                let two = format!("{}{}", curr, next.unwrap());

                // ??= is 3 chars but starts with ??
                if two == "??" && next2 == Some('=') {
                    if let Some(edit) = check_operator_spacing(&chars, i, 3, "??=") {
                        edits.push(edit);
                    }
                    i += 3;
                    continue;
                }

                // Skip heredoc/nowdoc operators (<<<)
                if two == "<<" && next2 == Some('<') {
                    i += 3;
                    continue;
                }

                if matches!(two.as_str(),
                    "==" | "!=" | "<=" | ">=" | "<>" | "??" |
                    "+=" | "-=" | "*=" | "/=" | ".=" | "%=" | "&=" | "|=" | "^=" | "**" |
                    "&&" | "||" | "=>" | "<<" | ">>"
                ) {
                    // Skip => inside declare()
                    if two == "=>" && is_in_declare(&source[..byte_pos]) {
                        i += 2;
                        continue;
                    }

                    // Skip ** in docblock start (/**)
                    if two == "**" && prev == Some('/') {
                        i += 2;
                        continue;
                    }

                    if let Some(edit) = check_operator_spacing(&chars, i, 2, &two) {
                        edits.push(edit);
                    }
                    i += 2;
                    continue;
                }
            }

            // Handle single-character operators: =, <, >, +, -, *, /, %, ., &, |, ^
            match curr {
                '=' => {
                    // Skip if part of multi-char operator (already handled above)
                    // Also skip in declare()
                    if is_in_declare(&source[..byte_pos]) {
                        i += 1;
                        continue;
                    }

                    // Make sure this isn't the start of == or =>
                    if next == Some('=') || next == Some('>') {
                        i += 1;
                        continue;
                    }

                    // Make sure this isn't preceded by an operator char (already handled as compound)
                    if matches!(prev, Some('+') | Some('-') | Some('*') | Some('/') | Some('.') |
                                Some('%') | Some('&') | Some('|') | Some('^') | Some('!') |
                                Some('<') | Some('>') | Some('?') | Some('=')) {
                        i += 1;
                        continue;
                    }

                    if let Some(edit) = check_operator_spacing(&chars, i, 1, "=") {
                        edits.push(edit);
                    }
                }
                '<' | '>' => {
                    // Skip if part of multi-char operator
                    if next == Some('=') || next == Some('>') || next == Some('<') {
                        i += 1;
                        continue;
                    }
                    // Skip PHP tags <?php and ?>
                    if curr == '<' && next == Some('?') {
                        i += 1;
                        continue;
                    }
                    // Skip object operator -> and nullsafe operator ?->
                    if curr == '>' && (prev == Some('-') || prev == Some('?')) {
                        i += 1;
                        continue;
                    }
                    // Skip array syntax $arr['key']
                    // Skip generic type hints like array<int>

                    let op = curr.to_string();
                    if let Some(edit) = check_operator_spacing(&chars, i, 1, &op) {
                        edits.push(edit);
                    }
                }
                '+' | '-' => {
                    // Skip if part of compound operator +=, -=
                    if next == Some('=') {
                        i += 1;
                        continue;
                    }
                    // Skip -> and ?-> (handled separately)
                    if curr == '-' && next == Some('>') {
                        i += 1;
                        continue;
                    }
                    // Skip increment/decrement operators ++, --
                    if next == Some(curr) || prev == Some(curr) {
                        i += 1;
                        continue;
                    }
                    // Skip unary operators (after operator, opening paren/bracket, or at start of expression)
                    // Unary: after ( [ { , ; = : ? ! & | < >
                    if matches!(prev, Some('(') | Some('[') | Some('{') | Some(',') | Some(';') |
                                Some('=') | Some(':') | Some('?') | Some('!') | Some('&') |
                                Some('|') | Some('<') | Some('>') | Some('\n') | None) {
                        i += 1;
                        continue;
                    }
                    // Also skip if prev is another operator
                    if matches!(prev, Some('+') | Some('-') | Some('*') | Some('/') | Some('%')) {
                        i += 1;
                        continue;
                    }
                    // Skip if prev is "return", "echo", etc. (these are unary)
                    // We'd need to look back further, so just check for whitespace before
                    // If we have whitespace before but not after, it's likely unary
                    let prev_is_space = prev.map(|c| c.is_whitespace()).unwrap_or(false);
                    let next_is_space = next.map(|c| c.is_whitespace()).unwrap_or(true);
                    if prev_is_space && !next_is_space {
                        // Check if we're after a keyword
                        let before_str = &source[..byte_pos];
                        if before_str.trim_end().ends_with("return") ||
                           before_str.trim_end().ends_with("echo") ||
                           before_str.trim_end().ends_with("print") ||
                           before_str.trim_end().ends_with("yield") {
                            i += 1;
                            continue;
                        }
                    }

                    let op = curr.to_string();
                    if let Some(edit) = check_operator_spacing(&chars, i, 1, &op) {
                        edits.push(edit);
                    }
                }
                '*' => {
                    // Skip if part of compound operator *=, **
                    if next == Some('=') || next == Some('*') || prev == Some('*') {
                        i += 1;
                        continue;
                    }
                    // Skip / * and * / (comment markers)
                    if prev == Some('/') || next == Some('/') {
                        i += 1;
                        continue;
                    }
                    // Skip splat operator (*$arr) and variadic (...$args, *$args)
                    if next == Some('$') || prev == Some('.') {
                        i += 1;
                        continue;
                    }

                    if let Some(edit) = check_operator_spacing(&chars, i, 1, "*") {
                        edits.push(edit);
                    }
                }
                '/' => {
                    // Skip if part of compound operator /=
                    if next == Some('=') {
                        i += 1;
                        continue;
                    }
                    // Skip comments // and /*
                    if next == Some('/') || next == Some('*') || prev == Some('*') || prev == Some('/') {
                        i += 1;
                        continue;
                    }

                    if let Some(edit) = check_operator_spacing(&chars, i, 1, "/") {
                        edits.push(edit);
                    }
                }
                '%' => {
                    // Skip if part of compound operator %=
                    if next == Some('=') {
                        i += 1;
                        continue;
                    }

                    if let Some(edit) = check_operator_spacing(&chars, i, 1, "%") {
                        edits.push(edit);
                    }
                }
                // Note: '.' (concatenation) is NOT handled by binary_operator_spaces
                // in PHP-CS-Fixer. Use concat_space fixer instead.
                _ => {}
            }

            i += 1;
        }

        edits
    }
}

/// Check if an operator at position `pos` with given length needs spacing, and return an Edit if so
fn check_operator_spacing(chars: &[char], pos: usize, op_len: usize, op: &str) -> Option<Edit> {
    let len = chars.len();
    let prev = if pos > 0 { Some(chars[pos - 1]) } else { None };
    let after_pos = pos + op_len;
    let after = if after_pos < len { Some(chars[after_pos]) } else { None };

    let needs_space_before = prev.map(|c| !c.is_whitespace()).unwrap_or(false);
    let needs_space_after = after.map(|c| !c.is_whitespace()).unwrap_or(false);

    if needs_space_before || needs_space_after {
        let replacement = if needs_space_before && needs_space_after {
            format!(" {} ", op)
        } else if needs_space_before {
            format!(" {}", op)
        } else {
            format!("{} ", op)
        };

        let byte_pos: usize = chars[..pos].iter().map(|c| c.len_utf8()).sum();
        let byte_end: usize = chars[..after_pos].iter().map(|c| c.len_utf8()).sum();

        Some(edit_with_rule(
            byte_pos,
            byte_end,
            replacement,
            format!("Add space around {} operator", op),
            "binary_operator_spaces",
        ))
    } else {
        None
    }
}

/// Check if we're inside a declare() statement (between 'declare(' and ')')
fn is_in_declare(before: &str) -> bool {
    let lower = before.to_lowercase();
    if let Some(declare_pos) = lower.rfind("declare(") {
        let after_declare = &before[declare_pos..];
        let mut depth = 0;
        for c in after_declare.chars() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    return false;
                }
            }
        }
        return depth > 0;
    }
    false
}

fn is_in_string_or_comment(before: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        if !in_single_quote && !in_double_quote && !in_block_comment {
            if c == '/' && prev_char == '/' {
                in_line_comment = true;
            }
            if c == '#' {
                in_line_comment = true;
            }
        }

        if !in_single_quote && !in_double_quote && !in_line_comment {
            if c == '*' && prev_char == '/' {
                in_block_comment = true;
            }
            if c == '/' && prev_char == '*' && in_block_comment {
                in_block_comment = false;
            }
        }

        if c == '\n' {
            in_line_comment = false;
        }

        if !in_line_comment && !in_block_comment {
            if c == '\'' && prev_char != '\\' && !in_double_quote {
                in_single_quote = !in_single_quote;
            }
            if c == '"' && prev_char != '\\' && !in_single_quote {
                in_double_quote = !in_double_quote;
            }
        }

        prev_char = c;
    }

    in_single_quote || in_double_quote || in_line_comment || in_block_comment
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        BinaryOperatorSpacesFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_assignment_correct() {
        let edits = check("<?php\n$a = 1;\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_assignment_no_space_before() {
        let source = "<?php\n$a= 1;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_assignment_no_space_after() {
        let source = "<?php\n$a =1;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_compound_plus_equals() {
        let source = "<?php\n$a+=1;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" += "));
    }

    #[test]
    fn test_compound_minus_equals() {
        let source = "<?php\n$a-=1;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" -= "));
    }

    #[test]
    fn test_compound_multiply_equals() {
        let source = "<?php\n$a*=2;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" *= "));
    }

    #[test]
    fn test_compound_divide_equals() {
        let source = "<?php\n$a/=2;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" /= "));
    }

    #[test]
    fn test_compound_concat_equals() {
        let source = "<?php\n$a.='str';\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" .= "));
    }

    #[test]
    fn test_compound_null_coalesce_equals() {
        let source = "<?php\n$a??='default';\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" ??= "));
    }

    #[test]
    fn test_comparison_double_equals() {
        let source = "<?php\nif ($a==$b) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" == "));
    }

    #[test]
    fn test_comparison_triple_equals() {
        let source = "<?php\nif ($a===$b) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" === "));
    }

    #[test]
    fn test_comparison_not_equals() {
        let source = "<?php\nif ($a!=$b) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" != "));
    }

    #[test]
    fn test_comparison_not_identical() {
        let source = "<?php\nif ($a!==$b) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" !== "));
    }

    #[test]
    fn test_comparison_less_than() {
        let source = "<?php\nif ($a<$b) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" < "));
    }

    #[test]
    fn test_comparison_greater_than() {
        let source = "<?php\nif ($a>$b) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" > "));
    }

    #[test]
    fn test_comparison_less_than_or_equal() {
        let source = "<?php\nif ($a<=$b) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" <= "));
    }

    #[test]
    fn test_comparison_greater_than_or_equal() {
        let source = "<?php\nif ($a>=$b) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" >= "));
    }

    #[test]
    fn test_arrow_in_foreach() {
        let source = "<?php\nforeach ($a as $k=>$v) {}\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" => "));
    }

    #[test]
    fn test_arrow_in_array() {
        let source = "<?php\n$a = ['key'=>'val'];\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        // Should have edit for =>
        assert!(edits.iter().any(|e| e.replacement.contains(" => ")));
    }

    #[test]
    fn test_concatenation_not_handled() {
        // Concatenation is handled by concat_space fixer, not binary_operator_spaces
        // Source with proper spacing around = but no spaces around .
        let source = "<?php\n$a = 'foo'.'bar';\n";
        let edits = check(source);
        // No edits - = already has spaces and . is not handled
        assert!(edits.is_empty(), "Expected no edits for concat, got: {:?}", edits);
    }

    #[test]
    fn test_float_unchanged() {
        // Float literals should not be split
        let edits = check("<?php\n$a = 1.5;\n");
        // No edits - = already has spaces
        assert!(edits.is_empty());
    }

    #[test]
    fn test_property_access_unchanged() {
        // Property access -> should not be treated as binary operator
        let edits = check("<?php\n$a = $obj->prop;\n");
        // No edits - = already has spaces
        assert!(edits.is_empty());
    }

    #[test]
    fn test_and_operator() {
        let source = "<?php\n$a&&$b;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" && "));
    }

    #[test]
    fn test_or_operator() {
        let source = "<?php\n$a||$b;\n";
        let edits = check(source);
        assert!(!edits.is_empty());
        assert!(edits[0].replacement.contains(" || "));
    }

    #[test]
    fn test_skip_declare_statement() {
        let edits = check("<?php\ndeclare(strict_types=1);\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let edits = check("<?php\n$a = 'b=c';\n");
        assert!(edits.is_empty());
    }

    #[test]
    fn test_operators_already_spaced() {
        let edits = check("<?php\n$a += 1; $b -= 1; $c == $d; $e => $f;\n");
        assert!(edits.is_empty());
    }
}
