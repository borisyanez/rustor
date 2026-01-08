//! Whitespace configuration types for PHP-CS-Fixer compatibility

use serde::{Deserialize, Serialize};

/// Indentation style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndentStyle {
    /// Use spaces for indentation
    Spaces(usize),
    /// Use tabs for indentation
    Tabs,
}

impl Default for IndentStyle {
    fn default() -> Self {
        IndentStyle::Spaces(4)
    }
}

impl IndentStyle {
    /// Get the indentation string for one level
    pub fn as_str(&self) -> &'static str {
        match self {
            IndentStyle::Spaces(2) => "  ",
            IndentStyle::Spaces(4) => "    ",
            IndentStyle::Spaces(n) => {
                // For non-standard sizes, we'll need to generate at runtime
                // This is a limitation - we use 4 spaces as fallback
                if *n <= 2 { "  " } else { "    " }
            }
            IndentStyle::Tabs => "\t",
        }
    }

    /// Get the number of spaces equivalent
    pub fn width(&self) -> usize {
        match self {
            IndentStyle::Spaces(n) => *n,
            IndentStyle::Tabs => 4, // Tab width for calculation purposes
        }
    }

    /// Parse from PHP-CS-Fixer config string
    /// e.g., "    " -> Spaces(4), "\t" -> Tabs
    pub fn from_php_config(s: &str) -> Self {
        if s == "\t" || s == "\\t" {
            IndentStyle::Tabs
        } else {
            let spaces = s.chars().filter(|c| *c == ' ').count();
            IndentStyle::Spaces(if spaces > 0 { spaces } else { 4 })
        }
    }
}

/// Line ending style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineEnding {
    /// Unix-style line endings (LF)
    Lf,
    /// Windows-style line endings (CRLF)
    CrLf,
}

impl Default for LineEnding {
    fn default() -> Self {
        LineEnding::Lf
    }
}

impl LineEnding {
    /// Get the line ending string
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }

    /// Parse from PHP-CS-Fixer config string
    /// e.g., "\n" -> Lf, "\r\n" -> CrLf
    pub fn from_php_config(s: &str) -> Self {
        if s.contains("\\r\\n") || s.contains("\r\n") {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        }
    }
}

/// Combined whitespace configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WhitespaceConfig {
    pub indent: IndentStyle,
    pub line_ending: LineEnding,
}

impl WhitespaceConfig {
    pub fn new(indent: IndentStyle, line_ending: LineEnding) -> Self {
        Self { indent, line_ending }
    }

    /// Create config for PSR-12 standard (4 spaces, LF)
    pub fn psr12() -> Self {
        Self {
            indent: IndentStyle::Spaces(4),
            line_ending: LineEnding::Lf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent_style_from_php() {
        assert_eq!(IndentStyle::from_php_config("    "), IndentStyle::Spaces(4));
        assert_eq!(IndentStyle::from_php_config("  "), IndentStyle::Spaces(2));
        assert_eq!(IndentStyle::from_php_config("\t"), IndentStyle::Tabs);
        assert_eq!(IndentStyle::from_php_config("\\t"), IndentStyle::Tabs);
    }

    #[test]
    fn test_line_ending_from_php() {
        assert_eq!(LineEnding::from_php_config("\\n"), LineEnding::Lf);
        assert_eq!(LineEnding::from_php_config("\n"), LineEnding::Lf);
        assert_eq!(LineEnding::from_php_config("\\r\\n"), LineEnding::CrLf);
        assert_eq!(LineEnding::from_php_config("\r\n"), LineEnding::CrLf);
    }

    #[test]
    fn test_indent_as_str() {
        assert_eq!(IndentStyle::Spaces(4).as_str(), "    ");
        assert_eq!(IndentStyle::Spaces(2).as_str(), "  ");
        assert_eq!(IndentStyle::Tabs.as_str(), "\t");
    }
}
