//! Issue/diagnostic types for static analysis results

use std::path::PathBuf;

/// Severity level for issues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Error - must be fixed
    Error,
    /// Warning - should be reviewed
    Warning,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

/// A single issue found during analysis
#[derive(Debug, Clone)]
pub struct Issue {
    /// The check that found this issue (e.g., "undefined.variable")
    pub check_id: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// File where the issue was found
    pub file: PathBuf,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// PHPStan-compatible identifier (e.g., "argument.type")
    pub identifier: Option<String>,
    /// Optional tip for fixing the issue
    pub tip: Option<String>,
}

impl Issue {
    /// Create a new error issue
    pub fn error(
        check_id: impl Into<String>,
        message: impl Into<String>,
        file: PathBuf,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            check_id: check_id.into(),
            severity: Severity::Error,
            message: message.into(),
            file,
            line,
            column,
            identifier: None,
            tip: None,
        }
    }

    /// Create a new warning issue
    pub fn warning(
        check_id: impl Into<String>,
        message: impl Into<String>,
        file: PathBuf,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            check_id: check_id.into(),
            severity: Severity::Warning,
            message: message.into(),
            file,
            line,
            column,
            identifier: None,
            tip: None,
        }
    }

    /// Add a PHPStan identifier
    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = Some(identifier.into());
        self
    }

    /// Add a tip for fixing
    pub fn with_tip(mut self, tip: impl Into<String>) -> Self {
        self.tip = Some(tip.into());
        self
    }
}

/// Collection of issues from analysis
#[derive(Debug, Default)]
pub struct IssueCollection {
    issues: Vec<Issue>,
}

impl IssueCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, issue: Issue) {
        self.issues.push(issue);
    }

    pub fn extend(&mut self, issues: impl IntoIterator<Item = Issue>) {
        self.issues.extend(issues);
    }

    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    pub fn into_issues(self) -> Vec<Issue> {
        self.issues
    }

    pub fn len(&self) -> usize {
        self.issues.len()
    }

    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    /// Sort issues by file, then line, then column
    pub fn sort(&mut self) {
        self.issues.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then_with(|| a.line.cmp(&b.line))
                .then_with(|| a.column.cmp(&b.column))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_creation() {
        let issue = Issue::error(
            "undefined.variable",
            "Undefined variable $foo",
            PathBuf::from("/test.php"),
            10,
            5,
        )
        .with_identifier("variable.undefined")
        .with_tip("Did you mean $foobar?");

        assert_eq!(issue.check_id, "undefined.variable");
        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.line, 10);
        assert_eq!(issue.identifier, Some("variable.undefined".to_string()));
    }

    #[test]
    fn test_issue_collection() {
        let mut collection = IssueCollection::new();
        collection.add(Issue::error("test", "Error 1", PathBuf::from("/a.php"), 1, 1));
        collection.add(Issue::warning("test", "Warning 1", PathBuf::from("/b.php"), 2, 1));

        assert_eq!(collection.len(), 2);
        assert_eq!(collection.error_count(), 1);
        assert_eq!(collection.warning_count(), 1);
    }
}
