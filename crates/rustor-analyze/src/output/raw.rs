//! Raw output format (PHPStan compatible)
//!
//! Format: file:line:message
//!
//! This matches PHPStan's raw format exactly - one error per line,
//! no headers, no summary, just file:line:message

use super::Formatter;
use crate::issue::IssueCollection;

pub struct RawFormatter;

impl Formatter for RawFormatter {
    fn format(&self, issues: &IssueCollection) -> String {
        let mut output = String::new();

        // Sort issues by file then line for consistent output
        let mut sorted_issues: Vec<_> = issues.issues().iter().collect();
        sorted_issues.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then_with(|| a.line.cmp(&b.line))
        });

        for issue in sorted_issues {
            // PHPStan raw format: file:line:message
            output.push_str(&format!(
                "{}:{}:{}\n",
                issue.file.display(),
                issue.line,
                issue.message
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
    fn test_raw_format() {
        let mut issues = IssueCollection::new();
        issues.add(Issue::error(
            "test",
            "Test error",
            PathBuf::from("/path/to/file.php"),
            10,
            5,
        ));

        let formatter = RawFormatter;
        let output = formatter.format(&issues);

        assert!(output.contains("/path/to/file.php:10:Test error"));
    }
}
