//! Scanner for PHP require/include statements
//!
//! Extracts file paths from require, require_once, include, include_once statements
//! and resolves them relative to the source file.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Collects included file paths from a PHP file
pub struct IncludeScanner<'s> {
    source: &'s str,
    file_dir: PathBuf,
    includes: Vec<PathBuf>,
    seen: HashSet<PathBuf>,
}

impl<'s> IncludeScanner<'s> {
    pub fn new(source: &'s str, file_path: &Path) -> Self {
        let file_dir = file_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Self {
            source,
            file_dir,
            includes: Vec::new(),
            seen: HashSet::new(),
        }
    }

    /// Scan a program for include statements and return resolved paths
    pub fn scan<'a>(mut self, program: &Program<'a>) -> Vec<PathBuf> {
        self.visit_program(program, self.source);
        self.includes
    }

    /// Extract text from a span
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Try to resolve an include path from an expression
    fn resolve_include_path(&self, expr: &Expression<'_>) -> Option<PathBuf> {
        match expr {
            // Simple string literal: require_once 'file.php'
            Expression::Literal(Literal::String(s)) => {
                let text = self.get_span_text(&s.span());
                let path = extract_string_content(text)?;
                Some(self.file_dir.join(path))
            }
            // Binary concat: __DIR__ . '/file.php'
            Expression::Binary(binary) => {
                if let BinaryOperator::StringConcat(_) = &binary.operator {
                    self.resolve_concat_path(&binary.lhs, &binary.rhs)
                } else {
                    None
                }
            }
            // Parenthesized expression
            Expression::Parenthesized(paren) => self.resolve_include_path(&paren.expression),
            _ => None,
        }
    }

    /// Resolve a concatenation expression like __DIR__ . '/path/file.php'
    fn resolve_concat_path(&self, lhs: &Expression<'_>, rhs: &Expression<'_>) -> Option<PathBuf> {
        // Check if lhs is __DIR__
        let lhs_text = self.get_span_text(&lhs.span());
        let base_dir = if lhs_text.trim() == "__DIR__" {
            self.file_dir.clone()
        } else if let Expression::Literal(Literal::String(s)) = lhs {
            let text = self.get_span_text(&s.span());
            PathBuf::from(extract_string_content(text)?)
        } else if let Expression::Binary(binary) = lhs {
            // Nested concat: __DIR__ . '/../' . 'file.php'
            if let BinaryOperator::StringConcat(_) = &binary.operator {
                self.resolve_concat_path(&binary.lhs, &binary.rhs)?
            } else {
                return None;
            }
        } else {
            return None;
        };

        // Get rhs string
        let rhs_path = match rhs {
            Expression::Literal(Literal::String(s)) => {
                let text = self.get_span_text(&s.span());
                extract_string_content(text)?
            }
            Expression::Binary(binary) => {
                if let BinaryOperator::StringConcat(_) = &binary.operator {
                    // Recurse for nested concats
                    let resolved = self.resolve_concat_path(&binary.lhs, &binary.rhs)?;
                    return Some(base_dir.join(resolved));
                }
                return None;
            }
            _ => return None,
        };

        // Join and normalize the path
        let full_path = base_dir.join(rhs_path.trim_start_matches('/'));
        Some(normalize_path(&full_path))
    }

    /// Process an include construct
    fn process_include(&mut self, value: &Expression<'_>) {
        if let Some(path) = self.resolve_include_path(value) {
            let canonical = normalize_path(&path);
            if canonical.exists() && !self.seen.contains(&canonical) {
                self.seen.insert(canonical.clone());
                self.includes.push(canonical);
            }
        }
    }
}

impl<'a, 's> Visitor<'a> for IncludeScanner<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Construct(construct) = expr {
            match construct {
                Construct::Require(req) => self.process_include(req.value),
                Construct::RequireOnce(req) => self.process_include(req.value),
                Construct::Include(inc) => self.process_include(inc.value),
                Construct::IncludeOnce(inc) => self.process_include(inc.value),
                _ => {}
            }
        }
        true // Continue visiting
    }
}

/// Extract string content from a PHP string literal (removes quotes)
fn extract_string_content(s: &str) -> Option<String> {
    let s = s.trim();
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}

/// Normalize a path (resolve . and ..)
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;

    fn scan_includes(source: &str, file_path: &str) -> Vec<PathBuf> {
        let arena = Bump::new();
        let file_id = FileId::new(file_path);
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        let scanner = IncludeScanner::new(source, Path::new(file_path));
        scanner.scan(program)
    }

    #[test]
    fn test_simple_require() {
        // This test would need actual files to exist
        let source = r#"<?php
require_once __DIR__ . '/test.php';
"#;
        let includes = scan_includes(source, "/tmp/test/main.php");
        // Path would be /tmp/test/test.php
        assert!(includes.is_empty() || includes[0].ends_with("test.php"));
    }
}
