//! Fixer implementations for PHP-CS-Fixer compatibility
//!
//! This module contains individual fixers that can be applied to PHP source code
//! to enforce formatting standards like PSR-12.

mod registry;
pub mod whitespace;
pub mod casing;
pub mod braces;
pub mod imports;
pub mod operators;
pub mod comments;
pub mod class;
pub mod functions;
pub mod risky;
pub mod syntax;
pub mod cleanup;
pub mod phpdoc;
pub mod types;
pub mod misc;

pub use registry::{FixerRegistry, FixerInfo};

use std::collections::HashMap;
use rustor_core::Edit;
use crate::config::{WhitespaceConfig, IndentStyle, LineEnding};

/// Configuration passed to fixers
#[derive(Debug, Clone)]
pub struct FixerConfig {
    /// Indentation style
    pub indent: IndentStyle,
    /// Line ending style
    pub line_ending: LineEnding,
    /// Rule-specific options
    pub options: HashMap<String, ConfigValue>,
}

impl Default for FixerConfig {
    fn default() -> Self {
        Self {
            indent: IndentStyle::default(),
            line_ending: LineEnding::default(),
            options: HashMap::new(),
        }
    }
}

impl From<&WhitespaceConfig> for FixerConfig {
    fn from(ws: &WhitespaceConfig) -> Self {
        Self {
            indent: ws.indent,
            line_ending: ws.line_ending,
            options: HashMap::new(),
        }
    }
}

/// Configuration value types for fixer options
#[derive(Debug, Clone)]
pub enum ConfigValue {
    Bool(bool),
    String(String),
    Number(i64),
    Array(Vec<String>),
    StringMap(std::collections::HashMap<String, String>),
}

/// A formatting fixer that can be applied to PHP source code
pub trait Fixer: Send + Sync {
    /// Internal name for this fixer
    fn name(&self) -> &'static str;

    /// PHP-CS-Fixer compatible name
    fn php_cs_fixer_name(&self) -> &'static str;

    /// Human-readable description
    fn description(&self) -> &'static str;

    /// Execution priority (higher = runs first)
    /// PHP-CS-Fixer uses 0-100 range, with common values:
    /// - 100: encoding
    /// - 90: opening tag
    /// - 70: whitespace
    /// - 50: indentation
    /// - 40: casing
    /// - 35: braces
    /// - 30: control structures
    /// - 20: imports, comments
    /// - 10: cleanup
    fn priority(&self) -> i32;

    /// Whether this fixer makes risky changes
    fn is_risky(&self) -> bool {
        false
    }

    /// Check the source and return edits to apply
    fn check(&self, source: &str, config: &FixerConfig) -> Vec<Edit>;

    /// Get configurable options for this fixer
    fn options(&self) -> Vec<FixerOption> {
        vec![]
    }
}

/// A configurable option for a fixer
#[derive(Debug, Clone)]
pub struct FixerOption {
    pub name: &'static str,
    pub description: &'static str,
    pub option_type: OptionType,
    pub default: Option<ConfigValue>,
}

/// Type of a fixer option
#[derive(Debug, Clone)]
pub enum OptionType {
    Bool,
    String,
    Number,
    StringArray,
    Enum(Vec<&'static str>),
}

/// Create an Edit with a rule name
pub fn edit_with_rule(start: usize, end: usize, replacement: String, message: String, rule: &str) -> Edit {
    use mago_span::{Position, Span};
    use mago_database::file::FileId;

    let span = Span::new(
        FileId::zero(),
        Position::new(start as u32),
        Position::new(end as u32),
    );

    Edit {
        span,
        replacement,
        message,
        rule: Some(rule.to_string()),
    }
}

/// Helper to calculate byte offset for a line and column
pub fn line_col_to_offset(source: &str, line: usize, col: usize) -> usize {
    let mut offset = 0;
    for (i, l) in source.lines().enumerate() {
        if i == line {
            return offset + col.min(l.len());
        }
        offset += l.len() + 1; // +1 for newline
    }
    offset
}

/// Get line number (0-indexed) for a byte offset
pub fn offset_to_line(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].matches('\n').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_col_to_offset() {
        let source = "line1\nline2\nline3";
        assert_eq!(line_col_to_offset(source, 0, 0), 0);
        assert_eq!(line_col_to_offset(source, 0, 3), 3);
        assert_eq!(line_col_to_offset(source, 1, 0), 6);
        assert_eq!(line_col_to_offset(source, 2, 0), 12);
    }

    #[test]
    fn test_offset_to_line() {
        let source = "line1\nline2\nline3";
        assert_eq!(offset_to_line(source, 0), 0);
        assert_eq!(offset_to_line(source, 5), 0);
        assert_eq!(offset_to_line(source, 6), 1);
        assert_eq!(offset_to_line(source, 12), 2);
    }
}
