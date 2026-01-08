//! GitHub Actions annotations output format

use super::Formatter;
use crate::issue::{IssueCollection, Severity};

pub struct GithubFormatter;

impl Formatter for GithubFormatter {
    fn format(&self, issues: &IssueCollection) -> String {
        let mut output = String::new();

        for issue in issues.issues() {
            let level = match issue.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };

            // GitHub Actions annotation format:
            // ::error file={name},line={line},col={col}::{message}
            output.push_str(&format!(
                "::{} file={},line={},col={}::{}\n",
                level,
                issue.file.display(),
                issue.line,
                issue.column,
                escape_message(&issue.message)
            ));
        }

        output
    }
}

/// Escape special characters for GitHub Actions annotations
fn escape_message(message: &str) -> String {
    message
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::Issue;
    use std::path::PathBuf;

    #[test]
    fn test_github_format() {
        let mut issues = IssueCollection::new();
        issues.add(Issue::error(
            "test",
            "Test error",
            PathBuf::from("src/file.php"),
            10,
            5,
        ));

        let formatter = GithubFormatter;
        let output = formatter.format(&issues);

        assert!(output.contains("::error file=src/file.php,line=10,col=5::Test error"));
    }

    #[test]
    fn test_escape_message() {
        assert_eq!(escape_message("line1\nline2"), "line1%0Aline2");
        assert_eq!(escape_message("100%"), "100%25");
    }
}
