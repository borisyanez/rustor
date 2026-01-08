//! Table output format (default, human-readable)

use super::Formatter;
use crate::issue::{IssueCollection, Severity};
use std::collections::HashMap;

pub struct TableFormatter;

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

            output.push_str(&format!("\n -- {} --\n\n", file_path));

            for issue in file_issues.iter() {
                let severity_marker = match issue.severity {
                    Severity::Error => "ERROR",
                    Severity::Warning => "WARNING",
                };

                output.push_str(&format!(
                    " {} Line {}: {}\n",
                    severity_marker,
                    issue.line,
                    issue.message
                ));

                if let Some(tip) = &issue.tip {
                    output.push_str(&format!("       Tip: {}\n", tip));
                }
            }
        }

        // Summary
        output.push_str(&format!(
            "\n [ERROR] Found {} error{}\n",
            issues.error_count(),
            if issues.error_count() == 1 { "" } else { "s" }
        ));

        if issues.warning_count() > 0 {
            output.push_str(&format!(
                " [WARNING] Found {} warning{}\n",
                issues.warning_count(),
                if issues.warning_count() == 1 { "" } else { "s" }
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
            PathBuf::from("/path/to/file.php"),
            10,
            5,
        ));

        let formatter = TableFormatter;
        let output = formatter.format(&issues);

        assert!(output.contains("file.php"));
        assert!(output.contains("Line 10"));
        assert!(output.contains("Test error"));
        assert!(output.contains("[ERROR] Found 1 error"));
    }
}
