//! Table output format (PHPStan compatible)
//!
//! Produces a table matching PHPStan's table format:
//! ```text
//!  ------ -----------------------------------------------------------------------
//!   Line   src/path/to/filename.php
//!  ------ -----------------------------------------------------------------------
//!   10     Error message here.
//!          🪪  error.identifier
//!  ------ -----------------------------------------------------------------------
//!
//!  [ERROR] Found N errors
//! ```

use super::Formatter;
use crate::issue::IssueCollection;
use std::collections::HashMap;

pub struct TableFormatter;

/// Width of the line number column (including padding)
const LINE_COL_WIDTH: usize = 6;
/// Width of the message column
const MSG_COL_WIDTH: usize = 71;

impl TableFormatter {
    /// Wrap text to fit within the message column width
    fn wrap_text(text: &str, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    /// Create a horizontal separator line
    fn separator() -> String {
        format!(
            " {:-<width$} {:-<msg_width$} \n",
            "",
            "",
            width = LINE_COL_WIDTH,
            msg_width = MSG_COL_WIDTH
        )
    }
}

impl Formatter for TableFormatter {
    fn format(&self, issues: &IssueCollection) -> String {
        if issues.is_empty() {
            return " [OK] No errors\n".to_string();
        }

        let mut output = String::new();

        // Group by file
        let mut files: HashMap<String, Vec<_>> = HashMap::new();
        for issue in issues.issues() {
            let path = issue.file.display().to_string();
            files.entry(path).or_default().push(issue);
        }

        // Sort files
        let mut file_list: Vec<_> = files.keys().collect();
        file_list.sort();

        for file_path in file_list {
            let file_issues = files.get(file_path).unwrap();

            // Use the full relative path for the header
            let filename = file_path.clone();

            // Table header
            output.push_str(&Self::separator());
            output.push_str(&format!(
                "  {:>width$}   {:<msg_width$}\n",
                "Line",
                filename,
                width = LINE_COL_WIDTH - 2,
                msg_width = MSG_COL_WIDTH
            ));
            output.push_str(&Self::separator());

            // Sort issues by line number
            let mut sorted_issues: Vec<_> = file_issues.iter().collect();
            sorted_issues.sort_by_key(|i| i.line);

            for issue in sorted_issues {
                // Wrap the message
                let wrapped = Self::wrap_text(&issue.message, MSG_COL_WIDTH);

                // First line with line number
                output.push_str(&format!(
                    "  {:>width$}   {:<msg_width$}\n",
                    issue.line,
                    wrapped[0],
                    width = LINE_COL_WIDTH - 2,
                    msg_width = MSG_COL_WIDTH
                ));

                // Continuation lines (message overflow)
                for line in wrapped.iter().skip(1) {
                    output.push_str(&format!(
                        "  {:>width$}   {:<msg_width$}\n",
                        "",
                        line,
                        width = LINE_COL_WIDTH - 2,
                        msg_width = MSG_COL_WIDTH
                    ));
                }

                // Identifier line (if present)
                if let Some(identifier) = &issue.identifier {
                    output.push_str(&format!(
                        "  {:>width$}   🪪  {:<msg_width$}\n",
                        "",
                        identifier,
                        width = LINE_COL_WIDTH - 2,
                        msg_width = MSG_COL_WIDTH - 4
                    ));
                }

                // Tip line (if present)
                if let Some(tip) = &issue.tip {
                    let tip_text = format!("💡 {}", tip);
                    output.push_str(&format!(
                        "  {:>width$}   {:<msg_width$}\n",
                        "",
                        tip_text,
                        width = LINE_COL_WIDTH - 2,
                        msg_width = MSG_COL_WIDTH
                    ));
                }
            }

            // Table footer
            output.push_str(&Self::separator());
        }

        // Summary
        output.push('\n');
        let error_count = issues.error_count();
        if error_count > 0 {
            output.push_str(&format!(
                " [ERROR] Found {} error{}\n",
                error_count,
                if error_count == 1 { "" } else { "s" }
            ));
        }

        let warning_count = issues.warning_count();
        if warning_count > 0 {
            output.push_str(&format!(
                " [WARNING] Found {} warning{}\n",
                warning_count,
                if warning_count == 1 { "" } else { "s" }
            ));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::Issue;
    use std::path::PathBuf;

    #[test]
    fn test_table_format_empty() {
        let issues = IssueCollection::new();
        let formatter = TableFormatter;
        let output = formatter.format(&issues);
        assert!(output.contains("[OK]"));
    }

    #[test]
    fn test_table_format_with_errors() {
        let mut issues = IssueCollection::new();
        issues.add(Issue::error(
            "test",
            "Test error",
            PathBuf::from("src/path/to/file.php"),
            10,
            5,
        ));

        let formatter = TableFormatter;
        let output = formatter.format(&issues);

        // Should contain the full relative path
        assert!(output.contains("src/path/to/file.php"));
        assert!(output.contains("10")); // Line number
        assert!(output.contains("Test error"));
        assert!(output.contains("[ERROR] Found 1 error"));
    }

    #[test]
    fn test_table_format_with_identifier() {
        let mut issues = IssueCollection::new();
        issues.add(
            Issue::error(
                "test",
                "Test error message",
                PathBuf::from("/path/to/file.php"),
                10,
                5,
            )
            .with_identifier("test.identifier"),
        );

        let formatter = TableFormatter;
        let output = formatter.format(&issues);

        assert!(output.contains("🪪"));
        assert!(output.contains("test.identifier"));
    }

    #[test]
    fn test_table_format_separator() {
        let sep = TableFormatter::separator();
        assert!(sep.contains("------"));
    }

    #[test]
    fn test_wrap_text() {
        let text = "This is a very long message that should be wrapped";
        let wrapped = TableFormatter::wrap_text(text, 20);
        assert!(wrapped.len() > 1);
        for line in &wrapped {
            assert!(line.len() <= 20);
        }
    }
}
