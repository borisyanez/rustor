//! Span-based source code editing with format preservation

use mago_span::Span;
use thiserror::Error;

/// Errors that can occur during edit application
#[derive(Error, Debug)]
pub enum EditError {
    #[error("Overlapping edits detected at offset {0}")]
    OverlappingEdits(usize),

    #[error("Edit span {start}..{end} out of bounds for source length {len}")]
    SpanOutOfBounds { start: usize, end: usize, len: usize },
}

/// Represents a single code edit operation
#[derive(Debug, Clone)]
pub struct Edit {
    /// The source span to replace
    pub span: Span,
    /// The replacement text
    pub replacement: String,
    /// Human-readable description of the edit
    pub message: String,
    /// Optional rule name for identification
    pub rule: Option<String>,
}

/// Represents a group of related edits that should be applied atomically
///
/// Used for complex refactorings that require multiple coordinated changes,
/// such as constructor promotion which needs to:
/// 1. Remove property declarations
/// 2. Modify constructor parameters
/// 3. Remove assignment statements
#[derive(Debug, Clone)]
pub struct EditGroup {
    /// All edits in this group
    pub edits: Vec<Edit>,
    /// Human-readable description of the group
    pub message: String,
    /// Rule name that generated this group
    pub rule: String,
}

impl EditGroup {
    /// Create a new edit group
    pub fn new(rule: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            edits: Vec::new(),
            message: message.into(),
            rule: rule.into(),
        }
    }

    /// Add an edit to the group
    pub fn add_edit(&mut self, span: Span, replacement: impl Into<String>) {
        self.edits.push(Edit {
            span,
            replacement: replacement.into(),
            message: String::new(),
            rule: Some(self.rule.clone()),
        });
    }

    /// Add an edit with a message
    pub fn add_edit_with_message(
        &mut self,
        span: Span,
        replacement: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.edits.push(Edit {
            span,
            replacement: replacement.into(),
            message: message.into(),
            rule: Some(self.rule.clone()),
        });
    }

    /// Check if this group is empty
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Get the number of edits in this group
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// Get the primary span (first edit's span) for reporting
    pub fn primary_span(&self) -> Option<Span> {
        self.edits.first().map(|e| e.span)
    }
}

impl Edit {
    /// Create a new edit
    pub fn new(span: Span, replacement: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            span,
            replacement: replacement.into(),
            message: message.into(),
            rule: None,
        }
    }

    /// Create a new edit with a rule name
    pub fn with_rule(
        span: Span,
        replacement: impl Into<String>,
        message: impl Into<String>,
        rule: impl Into<String>,
    ) -> Self {
        Self {
            span,
            replacement: replacement.into(),
            message: message.into(),
            rule: Some(rule.into()),
        }
    }

    /// Get the byte offset where this edit starts
    pub fn start_offset(&self) -> usize {
        self.span.start.offset as usize
    }

    /// Get the byte offset where this edit ends
    pub fn end_offset(&self) -> usize {
        self.span.end.offset as usize
    }
}

/// Apply edits to source code, preserving surrounding formatting
///
/// Edits are applied in reverse order (from end to start) to maintain
/// valid offsets throughout the process.
///
/// # Arguments
/// * `source` - The original source code
/// * `edits` - Slice of edits to apply
///
/// # Returns
/// * `Ok(String)` - The modified source code
/// * `Err(EditError)` - If edits overlap or are out of bounds
pub fn apply_edits(source: &str, edits: &[Edit]) -> Result<String, EditError> {
    if edits.is_empty() {
        return Ok(source.to_string());
    }

    // Sort edits by start position (descending) for safe replacement
    let mut sorted_edits: Vec<&Edit> = edits.iter().collect();
    sorted_edits.sort_by(|a, b| b.start_offset().cmp(&a.start_offset()));

    // Validate: check for overlapping edits and bounds
    let source_len = source.len();
    let mut prev_start: Option<usize> = None;

    for edit in &sorted_edits {
        let start = edit.start_offset();
        let end = edit.end_offset();

        // Check bounds
        if end > source_len {
            return Err(EditError::SpanOutOfBounds {
                start,
                end,
                len: source_len,
            });
        }

        // Check for overlap with previous edit
        if let Some(prev) = prev_start {
            if end > prev {
                return Err(EditError::OverlappingEdits(start));
            }
        }

        prev_start = Some(start);
    }

    // Apply edits from end to start
    let mut result = source.to_string();

    for edit in sorted_edits {
        let start = edit.start_offset();
        let end = edit.end_offset();

        // Get original text for whitespace analysis
        let original = &source[start..end];

        // Preserve leading whitespace from original
        let replacement = adjust_whitespace(original, &edit.replacement);

        result.replace_range(start..end, &replacement);
    }

    Ok(result)
}

/// Apply edit groups to source code atomically
///
/// Each group's edits are applied together. If any edit in a group fails,
/// the entire group is skipped.
///
/// # Arguments
/// * `source` - The original source code
/// * `groups` - Slice of edit groups to apply
///
/// # Returns
/// * `Ok(String)` - The modified source code
/// * `Err(EditError)` - If edits overlap or are out of bounds
pub fn apply_edit_groups(source: &str, groups: &[EditGroup]) -> Result<String, EditError> {
    // Flatten all edits from all groups
    let all_edits: Vec<Edit> = groups
        .iter()
        .flat_map(|g| g.edits.clone())
        .collect();

    apply_edits(source, &all_edits)
}

/// Attempt to preserve whitespace patterns from original code
fn adjust_whitespace(original: &str, replacement: &str) -> String {
    // Simple heuristic: preserve leading whitespace
    let leading_ws: String = original
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();

    if !leading_ws.is_empty() && !replacement.starts_with(&leading_ws) {
        format!("{}{}", leading_ws, replacement.trim_start())
    } else {
        replacement.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mago_database::file::FileId;
    use mago_span::{Position, Span};

    fn make_span(start: u32, end: u32) -> Span {
        let file_id = FileId::zero();
        Span::new(file_id, Position::new(start), Position::new(end))
    }

    #[test]
    fn test_simple_replacement() {
        let source = "array_push($arr, $val);";
        let edit = Edit::new(make_span(0, 22), "$arr[] = $val", "Replace array_push");

        let result = apply_edits(source, &[edit]).unwrap();
        assert_eq!(result, "$arr[] = $val;");
    }

    #[test]
    fn test_multiple_edits() {
        let source = "array_push($a, 1); array_push($b, 2);";
        let edits = vec![
            Edit::new(make_span(0, 17), "$a[] = 1", "first"),
            Edit::new(make_span(19, 36), "$b[] = 2", "second"),
        ];

        let result = apply_edits(source, &edits).unwrap();
        assert_eq!(result, "$a[] = 1; $b[] = 2;");
    }

    #[test]
    fn test_empty_edits() {
        let source = "unchanged";
        let result = apply_edits(source, &[]).unwrap();
        assert_eq!(result, "unchanged");
    }

    #[test]
    fn test_out_of_bounds() {
        let source = "short";
        let edit = Edit::new(make_span(0, 100), "replacement", "oob");

        let result = apply_edits(source, &[edit]);
        assert!(matches!(result, Err(EditError::SpanOutOfBounds { .. })));
    }
}
