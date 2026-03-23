//! PHPDoc parse error detection (Level 6)
//!
//! Detects malformed PHPDoc annotations:
//! - `@param` or `@return` or `@var` tag with no type specified
//! - `@param` tag with a type but missing the `$variable` name
//! - Unbalanced angle brackets `<>` in type expressions
//!
//! PHPStan message format: "PHPDoc tag @param has invalid value ..."

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::Program;
use std::path::PathBuf;

pub struct PhpDocParseErrorCheck;

impl Check for PhpDocParseErrorCheck {
    fn id(&self) -> &'static str {
        "phpDoc.parseError"
    }

    fn description(&self) -> &'static str {
        "Detects malformed PHPDoc annotations with invalid or missing type values"
    }

    fn level(&self) -> u8 {
        6
    }

    fn check<'a>(&self, _program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut checker = PhpDocErrorChecker {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };
        checker.scan();
        checker.issues
    }
}

struct PhpDocErrorChecker<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

impl<'s> PhpDocErrorChecker<'s> {
    fn line_col_at(&self, offset: usize) -> (usize, usize) {
        let mut line = 1usize;
        let mut col = 1usize;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    fn scan(&mut self) {
        // Find all /** ... */ blocks
        let mut search_from = 0;
        while let Some(rel_start) = self.source[search_from..].find("/**") {
            let doc_start = search_from + rel_start;

            // Find the closing */
            let after_open = doc_start + 3;
            if let Some(rel_end) = self.source[after_open..].find("*/") {
                let doc_end = after_open + rel_end + 2; // include the */
                let doc_text = &self.source[doc_start..doc_end];

                // Compute line number of the /** opening
                let doc_line_base = self.count_lines_before(doc_start);

                self.check_phpdoc(doc_text, doc_start, doc_line_base);

                search_from = doc_end;
            } else {
                // No closing */ — stop scanning
                break;
            }
        }
    }

    fn count_lines_before(&self, offset: usize) -> usize {
        self.source[..offset].chars().filter(|&c| c == '\n').count() + 1
    }

    fn check_phpdoc(&mut self, doc: &str, doc_global_offset: usize, _doc_line_base: usize) {
        // Process each line of the PHPDoc looking for annotation tags
        let mut line_offset = doc_global_offset;
        for raw_line in doc.lines() {
            // Strip leading /** / * chars, then strip trailing */ chars
            let trimmed = raw_line
                .trim()
                .trim_start_matches(['/', '*', ' '])
                .trim_end_matches(['/', '*', ' ']);

            self.check_annotation_line(trimmed, line_offset);

            // Advance offset: +1 for the newline character
            line_offset += raw_line.len() + 1;
        }
    }

    fn check_annotation_line(&mut self, line: &str, _line_global_offset: usize) {
        // We report at the position of the tag in the original source, but since
        // we're doing text scanning, we'll use line/col computed from source offset.
        // For simplicity we compute line/col of the full doc offset passed.

        if let Some(rest) = line.strip_prefix("@param") {
            self.check_param_tag(rest.trim(), _line_global_offset, "@param");
        } else if let Some(rest) = line.strip_prefix("@return") {
            self.check_type_only_tag(rest.trim(), _line_global_offset, "@return");
        } else if let Some(rest) = line.strip_prefix("@var") {
            self.check_type_only_tag(rest.trim(), _line_global_offset, "@var");
        } else if let Some(rest) = line.strip_prefix("@throws") {
            self.check_type_only_tag(rest.trim(), _line_global_offset, "@throws");
        }
    }

    /// Check @param tag: must have a type AND a $variable name
    fn check_param_tag(&mut self, rest: &str, offset: usize, tag: &str) {
        if rest.is_empty() {
            let (line, col) = self.line_col_at(offset);
            self.issues.push(
                Issue::warning(
                    "phpDoc.parseError",
                    format!("PHPDoc tag {} has no type or variable name specified.", tag),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("phpDoc.parseError"),
            );
            return;
        }

        // Check for unbalanced angle brackets in the type portion
        if let Some(type_part) = self.extract_type_part(rest) {
            if self.has_unbalanced_brackets(&type_part) {
                let (line, col) = self.line_col_at(offset);
                self.issues.push(
                    Issue::warning(
                        "phpDoc.parseError",
                        format!(
                            "PHPDoc tag {} has invalid value (unbalanced angle brackets): {}",
                            tag, type_part
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("phpDoc.parseError"),
                );
                return;
            }
        }

        // @param must include a $variable name
        // It's valid to have a type without $var in some edge cases (like closures)
        // so only flag when the entire rest has no $ at all and looks like just a description
        if !rest.contains('$') && !rest.is_empty() {
            // Only flag if it doesn't look like a type (i.e., it's just prose)
            // Skip if it starts with something that looks like a type identifier
            let first_token = rest.split_whitespace().next().unwrap_or("");
            let looks_like_type = first_token.chars().next()
                .map(|c| c.is_uppercase() || c == '\\' || c == '?' || c == '(')
                .unwrap_or(false)
                || first_token == "int"
                || first_token == "string"
                || first_token == "bool"
                || first_token == "float"
                || first_token == "array"
                || first_token == "object"
                || first_token == "mixed"
                || first_token == "null"
                || first_token == "void"
                || first_token == "never"
                || first_token == "callable"
                || first_token == "iterable"
                || first_token == "self"
                || first_token == "static"
                || first_token == "false"
                || first_token == "true";

            if looks_like_type {
                // Has a type but no variable — flag it
                let (line, col) = self.line_col_at(offset);
                self.issues.push(
                    Issue::warning(
                        "phpDoc.parseError",
                        format!(
                            "PHPDoc tag {} has no variable name (missing $parameter): {}",
                            tag, rest
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("phpDoc.parseError"),
                );
            }
        }
    }

    /// Check @return / @var / @throws tag: must have a non-empty type
    fn check_type_only_tag(&mut self, rest: &str, offset: usize, tag: &str) {
        if rest.is_empty() {
            let (line, col) = self.line_col_at(offset);
            self.issues.push(
                Issue::warning(
                    "phpDoc.parseError",
                    format!("PHPDoc tag {} has no type specified.", tag),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("phpDoc.parseError"),
            );
            return;
        }

        // Check for unbalanced brackets
        let type_part = self.extract_type_part(rest).unwrap_or_else(|| rest.to_string());
        if self.has_unbalanced_brackets(&type_part) {
            let (line, col) = self.line_col_at(offset);
            self.issues.push(
                Issue::warning(
                    "phpDoc.parseError",
                    format!(
                        "PHPDoc tag {} has invalid value (unbalanced angle brackets): {}",
                        tag, type_part
                    ),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("phpDoc.parseError"),
            );
        }
    }

    /// Extract the type portion from an annotation value (everything up to whitespace after balanced type)
    fn extract_type_part(&self, s: &str) -> Option<String> {
        if s.is_empty() {
            return None;
        }
        let mut depth = 0i32;
        let mut end = s.len();
        for (i, ch) in s.char_indices() {
            match ch {
                '<' | '(' | '{' => depth += 1,
                '>' | ')' | '}' => depth -= 1,
                ' ' | '\t' if depth == 0 => {
                    end = i;
                    break;
                }
                _ => {}
            }
        }
        Some(s[..end].to_string())
    }

    /// Check if a type string has unbalanced angle brackets
    fn has_unbalanced_brackets(&self, s: &str) -> bool {
        let mut depth = 0i32;
        for ch in s.chars() {
            match ch {
                '<' => depth += 1,
                '>' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return true;
            }
        }
        depth != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn run_check(source: &str) -> Vec<Issue> {
        let mut checker = PhpDocErrorChecker {
            source,
            file_path: PathBuf::from("test.php"),
            issues: Vec::new(),
        };
        checker.scan();
        checker.issues
    }

    #[test]
    fn test_empty_param_tag() {
        let source = "<?php\n/** @param */\nfunction foo() {}";
        let issues = run_check(source);
        assert!(!issues.is_empty(), "Should detect empty @param tag");
        assert!(issues[0].check_id == "phpDoc.parseError");
    }

    #[test]
    fn test_empty_return_tag() {
        let source = "<?php\n/** @return */\nfunction foo() {}";
        let issues = run_check(source);
        assert!(!issues.is_empty(), "Should detect empty @return tag");
    }

    #[test]
    fn test_valid_param_tag() {
        let source = "<?php\n/** @param string $name Description */\nfunction foo(string $name) {}";
        let issues = run_check(source);
        assert!(issues.is_empty(), "Should not flag valid @param");
    }

    #[test]
    fn test_valid_return_tag() {
        let source = "<?php\n/** @return int */\nfunction foo(): int { return 1; }";
        let issues = run_check(source);
        assert!(issues.is_empty(), "Should not flag valid @return");
    }

    #[test]
    fn test_param_missing_variable() {
        let source = "<?php\n/** @param string */\nfunction foo(string $x) {}";
        let issues = run_check(source);
        assert!(!issues.is_empty(), "Should detect @param with type but no variable");
    }

    #[test]
    fn test_check_id_and_level() {
        assert_eq!(PhpDocParseErrorCheck.id(), "phpDoc.parseError");
        assert_eq!(PhpDocParseErrorCheck.level(), 6);
    }
}
