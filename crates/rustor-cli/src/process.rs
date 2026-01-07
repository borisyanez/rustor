//! File processing logic for rustor

use anyhow::{Context, Result};
use bumpalo::Bump;
use mago_database::file::FileId;
use std::collections::HashSet;
use std::path::Path;

use rustor_core::apply_edits;
use rustor_rules::RuleRegistry;

use crate::output::EditInfo;

/// Result of processing a single file
pub struct ProcessResult {
    /// Edits that were found/applied
    pub edits: Vec<EditInfo>,
    /// Original source code
    pub old_source: String,
    /// New source code after edits (only if edits were found)
    pub new_source: Option<String>,
}

/// Process a single PHP file and return the edits found
pub fn process_file(
    path: &Path,
    enabled_rules: &HashSet<String>,
) -> Result<Option<ProcessResult>> {
    let source_code = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    // Create arena allocator and file ID for mago
    let arena = Bump::new();
    let file_id = FileId::new(path.to_string_lossy().as_ref());

    // Parse the PHP file
    let (program, parse_error) =
        mago_syntax::parser::parse_file_content(&arena, file_id, &source_code);

    // Check for parse errors
    if let Some(_error) = parse_error {
        return Ok(None); // Signal parse error by returning None
    }

    // Apply enabled refactoring rules using the registry
    let registry = RuleRegistry::new();
    let edits = registry.check_all(program, &source_code, enabled_rules);

    if edits.is_empty() {
        return Ok(Some(ProcessResult {
            edits: vec![],
            old_source: source_code,
            new_source: None,
        }));
    }

    // Convert Edit to EditInfo with line/column info
    let edit_infos: Vec<EditInfo> = edits
        .iter()
        .map(|edit| {
            let (line, column) = offset_to_line_column(&source_code, edit.span.start.offset as usize);
            EditInfo {
                rule: extract_rule_name(&edit.message),
                line,
                column,
                message: edit.message.clone(),
            }
        })
        .collect();

    // Apply edits to get new source
    let new_source = apply_edits(&source_code, &edits)
        .with_context(|| format!("Failed to apply edits to {}", path.display()))?;

    Ok(Some(ProcessResult {
        edits: edit_infos,
        old_source: source_code,
        new_source: Some(new_source),
    }))
}

/// Write the processed result to the file
pub fn write_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write file: {}", path.display()))
}

/// Convert byte offset to line and column numbers (1-based)
fn offset_to_line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;

    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}

/// Extract rule name from edit message (heuristic)
fn extract_rule_name(message: &str) -> String {
    // Messages typically start with "Convert X" or "Replace X"
    let lower = message.to_lowercase();
    if lower.contains("array_push") {
        "array_push".to_string()
    } else if lower.contains("array()") || lower.contains("short array") {
        "array_syntax".to_string()
    } else if lower.contains("empty") && lower.contains("?:") {
        "empty_coalesce".to_string()
    } else if lower.contains("is_null") {
        "is_null".to_string()
    } else if lower.contains("isset") || lower.contains("??") {
        "isset_coalesce".to_string()
    } else if lower.contains("join") && lower.contains("implode") {
        "join_to_implode".to_string()
    } else if lower.contains("list") && lower.contains("[]") {
        "list_short_syntax".to_string()
    } else if lower.contains("pow") && lower.contains("**") {
        "pow_to_operator".to_string()
    } else if lower.contains("sizeof") || lower.contains("count") {
        "sizeof".to_string()
    } else if lower.contains("strval")
        || lower.contains("intval")
        || lower.contains("floatval")
        || lower.contains("boolval")
        || lower.contains("(string)")
        || lower.contains("(int)")
        || lower.contains("(float)")
        || lower.contains("(bool)")
    {
        "type_cast".to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_line_column() {
        let source = "line1\nline2\nline3";
        assert_eq!(offset_to_line_column(source, 0), (1, 1));
        assert_eq!(offset_to_line_column(source, 5), (1, 6)); // newline
        assert_eq!(offset_to_line_column(source, 6), (2, 1)); // start of line2
        assert_eq!(offset_to_line_column(source, 12), (3, 1)); // start of line3
    }

    #[test]
    fn test_extract_rule_name() {
        assert_eq!(
            extract_rule_name("Convert array_push() to short syntax"),
            "array_push"
        );
        assert_eq!(
            extract_rule_name("Convert array() to [] (short array syntax)"),
            "array_syntax"
        );
        assert_eq!(
            extract_rule_name("Convert is_null($x) to $x === null"),
            "is_null"
        );
        assert_eq!(
            extract_rule_name("Convert sizeof() to count()"),
            "sizeof"
        );
        assert_eq!(
            extract_rule_name("Replace join() with implode()"),
            "join_to_implode"
        );
    }
}
