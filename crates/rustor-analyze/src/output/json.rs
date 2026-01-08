//! JSON output format (PHPStan compatible)

use super::Formatter;
use crate::issue::{Issue, IssueCollection, Severity};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct JsonFormatter;

#[derive(Serialize)]
struct JsonOutput {
    totals: Totals,
    files: HashMap<String, FileErrors>,
    errors: Vec<String>,
}

#[derive(Serialize)]
struct Totals {
    errors: usize,
    file_errors: usize,
}

#[derive(Serialize)]
struct FileErrors {
    errors: usize,
    messages: Vec<FileMessage>,
}

#[derive(Serialize)]
struct FileMessage {
    message: String,
    line: usize,
    ignorable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tip: Option<String>,
}

impl Formatter for JsonFormatter {
    fn format(&self, issues: &IssueCollection) -> String {
        let mut files: HashMap<String, Vec<&Issue>> = HashMap::new();

        // Group issues by file
        for issue in issues.issues() {
            let path = issue.file.display().to_string();
            files.entry(path).or_default().push(issue);
        }

        // Build output structure
        let mut file_errors: HashMap<String, FileErrors> = HashMap::new();
        let mut total_errors = 0;

        for (path, path_issues) in files {
            let error_count = path_issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
                .count();
            total_errors += error_count;

            let messages: Vec<FileMessage> = path_issues
                .iter()
                .map(|issue| FileMessage {
                    message: issue.message.clone(),
                    line: issue.line,
                    ignorable: true,
                    identifier: issue.identifier.clone(),
                    tip: issue.tip.clone(),
                })
                .collect();

            file_errors.insert(
                path,
                FileErrors {
                    errors: error_count,
                    messages,
                },
            );
        }

        let output = JsonOutput {
            totals: Totals {
                errors: total_errors,
                file_errors: file_errors.len(),
            },
            files: file_errors,
            errors: vec![],
        };

        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_format() {
        let mut issues = IssueCollection::new();
        issues.add(Issue::error(
            "test",
            "Test error",
            PathBuf::from("/path/to/file.php"),
            10,
            5,
        ));

        let formatter = JsonFormatter;
        let output = formatter.format(&issues);

        assert!(output.contains("\"errors\": 1"));
        assert!(output.contains("Test error"));
    }
}
