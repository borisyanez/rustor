//! Output formatters for analysis results

mod raw;
mod json;
mod table;
mod github;

pub use raw::RawFormatter;
pub use json::JsonFormatter;
pub use table::TableFormatter;
pub use github::GithubFormatter;

use crate::issue::IssueCollection;

/// Output format for analysis results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// PHPStan raw format: file:line:message
    Raw,
    /// PHPStan JSON format
    Json,
    /// Table format (default)
    Table,
    /// GitHub Actions annotations
    Github,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "raw" => Some(OutputFormat::Raw),
            "json" => Some(OutputFormat::Json),
            "table" => Some(OutputFormat::Table),
            "github" => Some(OutputFormat::Github),
            _ => None,
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Table
    }
}

/// Trait for output formatters
pub trait Formatter {
    /// Format the issues and return the output string
    fn format(&self, issues: &IssueCollection) -> String;
}

/// Format issues using the specified format
pub fn format_issues(issues: &IssueCollection, format: OutputFormat) -> String {
    match format {
        OutputFormat::Raw => RawFormatter.format(issues),
        OutputFormat::Json => JsonFormatter.format(issues),
        OutputFormat::Table => TableFormatter.format(issues),
        OutputFormat::Github => GithubFormatter.format(issues),
    }
}
