//! Baseline support for gradual adoption
//!
//! Baselines allow ignoring existing errors so only new errors are reported.
//! This module is compatible with PHPStan's baseline format.

use crate::config::neon::{NeonParser, Value};
use crate::issue::IssueCollection;
use crate::logging;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A baseline entry representing an ignored error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineEntry {
    /// The error message (can be regex pattern like #^...$# or plain text)
    pub message: String,
    /// Number of occurrences of this error
    pub count: usize,
    /// File path where the error occurs
    pub path: String,
    /// Optional error identifier (e.g., "argument.type")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    /// Whether the message is a regex pattern
    #[serde(skip)]
    pub is_regex: bool,
    /// Compiled regex for matching (if is_regex)
    #[serde(skip)]
    compiled_regex: Option<Regex>,
}

impl BaselineEntry {
    /// Create a new baseline entry
    pub fn new(message: String, count: usize, path: String, identifier: Option<String>) -> Self {
        let is_regex = message.starts_with("#^") || message.starts_with("'#^") || message.starts_with("\"#^");
        let compiled_regex = if is_regex {
            Self::compile_regex(&message)
        } else {
            None
        };

        Self {
            message,
            count,
            path,
            identifier,
            is_regex,
            compiled_regex,
        }
    }

    /// Compile a PHPStan regex pattern
    fn compile_regex(pattern: &str) -> Option<Regex> {
        // Remove quotes and delimiters: '#^...$#' or "#^...$#" -> ...
        let inner = pattern
            .trim_matches(|c| c == '\'' || c == '"')
            .trim_start_matches("#^")
            .trim_end_matches("$#");

        // PHPStan uses PCRE, we use Rust regex - most patterns are compatible
        Regex::new(inner).ok()
    }

    /// Check if this entry matches an error message
    pub fn matches_message(&self, message: &str) -> bool {
        if let Some(ref regex) = self.compiled_regex {
            regex.is_match(message)
        } else {
            // Plain text matching
            self.message == message || message.contains(&self.message)
        }
    }

    /// Check if this entry matches an identifier
    pub fn matches_identifier(&self, identifier: Option<&str>) -> bool {
        match (&self.identifier, identifier) {
            (Some(pattern_id), Some(error_id)) => pattern_id == error_id,
            (None, _) => true, // No identifier specified means match all
            (Some(_), None) => false, // Pattern has identifier but error doesn't
        }
    }

    /// Check if this entry matches a file path
    pub fn matches_path(&self, file_path: &str) -> bool {
        // Normalize path separators
        let normalized_pattern = self.path.replace('\\', "/");
        let normalized_path = file_path.replace('\\', "/");

        // Check if the path ends with the pattern (relative path matching)
        normalized_path.ends_with(&normalized_pattern) ||
            normalized_path.contains(&normalized_pattern) ||
            normalized_pattern == normalized_path
    }
}

/// Baseline file structure
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Baseline {
    #[serde(default)]
    pub entries: Vec<BaselineEntry>,
}

impl Baseline {
    /// Create a new empty baseline
    pub fn new() -> Self {
        Self::default()
    }

    /// Load baseline from a NEON file (PHPStan format)
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(path)?;
        logging::log(&format!("Loading baseline from: {}", path.display()));
        Self::parse_neon(&content)
    }

    /// Parse NEON baseline format (PHPStan compatible)
    fn parse_neon(content: &str) -> Result<Self, std::io::Error> {
        let mut parser = NeonParser::new(content);

        let value = parser.parse().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, format!("NEON parse error: {}", e))
        })?;

        let mut entries = Vec::new();

        // Navigate: parameters -> ignoreErrors -> [array of entries]
        if let Some(params) = value.get("parameters") {
            if let Some(ignore_errors) = params.get("ignoreErrors") {
                if let Some(arr) = ignore_errors.as_array() {
                    for item in arr {
                        if let Some(entry) = Self::parse_entry(item) {
                            logging::log(&format!(
                                "  Baseline entry: {} (count: {}, path: {}{})",
                                &entry.message[..entry.message.len().min(60)],
                                entry.count,
                                entry.path,
                                entry.identifier.as_ref().map(|id| format!(", id: {}", id)).unwrap_or_default()
                            ));
                            entries.push(entry);
                        }
                    }
                }
            }
        }

        logging::log(&format!("Loaded {} baseline entries", entries.len()));
        Ok(Baseline { entries })
    }

    /// Parse a single baseline entry from NEON value
    fn parse_entry(value: &Value) -> Option<BaselineEntry> {
        let obj = value.as_object()?;

        let message = obj.get("message")?.as_str()?.to_string();
        let count = obj.get("count")
            .and_then(|v| v.as_i64())
            .map(|n| n as usize)
            .unwrap_or(1);
        let path = obj.get("path")?.as_str()?.to_string();
        let identifier = obj.get("identifier")
            .and_then(|v| v.as_str())
            .map(String::from);

        Some(BaselineEntry::new(message, count, path, identifier))
    }

    /// Generate a baseline from a collection of issues
    pub fn generate(issues: &IssueCollection) -> Self {
        // Group by (path, message, identifier)
        let mut grouped: HashMap<(String, String, Option<String>), usize> = HashMap::new();

        for issue in issues.issues() {
            let path = issue.file.display().to_string();
            let key = (path, issue.message.clone(), issue.identifier.clone());
            *grouped.entry(key).or_insert(0) += 1;
        }

        // Sort entries by path, then message for consistent output
        let mut entries: Vec<_> = grouped
            .into_iter()
            .map(|((path, message, identifier), count)| {
                // Create regex pattern for the message
                let regex_message = format!("#^{}$#", escape_regex(&message));
                BaselineEntry::new(regex_message, count, path, identifier)
            })
            .collect();

        entries.sort_by(|a, b| {
            a.path.cmp(&b.path).then_with(|| a.message.cmp(&b.message))
        });

        Baseline { entries }
    }

    /// Save baseline to a file (PHPStan NEON format)
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let content = self.to_neon();
        fs::write(path, content)
    }

    /// Convert to PHPStan-compatible NEON format
    fn to_neon(&self) -> String {
        let mut output = String::from("parameters:\n\tignoreErrors:\n");

        for entry in &self.entries {
            output.push_str("\t\t-\n");
            // Escape message for NEON
            let escaped_message = escape_neon_string(&entry.message);
            output.push_str(&format!("\t\t\tmessage: '{}'\n", escaped_message));
            if let Some(ref id) = entry.identifier {
                output.push_str(&format!("\t\t\tidentifier: {}\n", id));
            }
            output.push_str(&format!("\t\t\tcount: {}\n", entry.count));
            output.push_str(&format!("\t\t\tpath: {}\n", entry.path));
            output.push('\n');
        }

        output
    }

    /// Filter issues against the baseline
    pub fn filter(&self, issues: IssueCollection) -> IssueCollection {
        // Track remaining counts for each entry
        let mut remaining_counts: Vec<usize> = self.entries.iter().map(|e| e.count).collect();

        let mut filtered = IssueCollection::new();

        for issue in issues.into_issues() {
            let file_path = issue.file.display().to_string();
            let mut matched = false;

            // Try to match against baseline entries
            for (i, entry) in self.entries.iter().enumerate() {
                if remaining_counts[i] == 0 {
                    continue;
                }

                // Check path match
                if !entry.matches_path(&file_path) {
                    continue;
                }

                // Check identifier match
                if !entry.matches_identifier(issue.identifier.as_deref()) {
                    continue;
                }

                // Check message match
                if entry.matches_message(&issue.message) {
                    remaining_counts[i] -= 1;
                    matched = true;
                    logging::log(&format!(
                        "BASELINE FILTERED: {}:{} - {}",
                        file_path,
                        issue.line,
                        &issue.message[..issue.message.len().min(60)]
                    ));
                    break;
                }
            }

            if !matched {
                filtered.add(issue);
            }
        }

        filtered
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if baseline is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Escape special regex characters for use in a regex pattern
fn escape_regex(s: &str) -> String {
    let special_chars = ['\\', '.', '+', '*', '?', '(', ')', '[', ']', '{', '}', '^', '$', '|'];
    let mut result = String::with_capacity(s.len() * 2);

    for c in s.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }

    result
}

/// Escape special characters for NEON single-quoted string
fn escape_neon_string(s: &str) -> String {
    // In NEON single-quoted strings, only ' needs escaping as ''
    s.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::Issue;
    use std::path::PathBuf;

    #[test]
    fn test_baseline_entry_regex_matching() {
        let entry = BaselineEntry::new(
            "#^Call to undefined function foo\\(\\)\\.$#".to_string(),
            1,
            "test.php".to_string(),
            None,
        );

        assert!(entry.is_regex);
        assert!(entry.matches_message("Call to undefined function foo()."));
        assert!(!entry.matches_message("Call to undefined function bar()."));
    }

    #[test]
    fn test_baseline_entry_identifier_matching() {
        let entry = BaselineEntry::new(
            "Some error".to_string(),
            1,
            "test.php".to_string(),
            Some("argument.type".to_string()),
        );

        assert!(entry.matches_identifier(Some("argument.type")));
        assert!(!entry.matches_identifier(Some("other.type")));
        assert!(!entry.matches_identifier(None));
    }

    #[test]
    fn test_baseline_entry_path_matching() {
        let entry = BaselineEntry::new(
            "Error".to_string(),
            1,
            "src/Controller/FooController.php".to_string(),
            None,
        );

        assert!(entry.matches_path("src/Controller/FooController.php"));
        assert!(entry.matches_path("/project/src/Controller/FooController.php"));
        assert!(!entry.matches_path("src/Controller/BarController.php"));
    }

    #[test]
    fn test_parse_phpstan_baseline() {
        let content = r#"
parameters:
    ignoreErrors:
        -
            message: '#^Call to undefined function foo\(\)\.$#'
            identifier: function.notFound
            count: 2
            path: src/test.php
        -
            message: '#^Variable \$bar might not be defined\.$#'
            count: 1
            path: src/other.php
"#;

        let baseline = Baseline::parse_neon(content).unwrap();
        assert_eq!(baseline.entries.len(), 2);

        assert!(baseline.entries[0].is_regex);
        assert_eq!(baseline.entries[0].count, 2);
        assert_eq!(baseline.entries[0].identifier, Some("function.notFound".to_string()));

        assert_eq!(baseline.entries[1].count, 1);
        assert_eq!(baseline.entries[1].identifier, None);
    }

    #[test]
    fn test_generate_baseline() {
        let mut issues = IssueCollection::new();
        issues.add(Issue::error(
            "function.notFound",
            "Call to undefined function foo().",
            PathBuf::from("file.php"),
            10,
            1,
        ).with_identifier("function.notFound"));
        issues.add(Issue::error(
            "function.notFound",
            "Call to undefined function foo().",
            PathBuf::from("file.php"),
            20,
            1,
        ).with_identifier("function.notFound"));

        let baseline = Baseline::generate(&issues);
        assert_eq!(baseline.entries.len(), 1);
        assert_eq!(baseline.entries[0].count, 2);
    }

    #[test]
    fn test_filter_baseline() {
        let baseline = Baseline {
            entries: vec![BaselineEntry::new(
                "#^Error 1$#".to_string(),
                1,
                "file.php".to_string(),
                None,
            )],
        };

        let mut issues = IssueCollection::new();
        issues.add(Issue::error(
            "test",
            "Error 1",
            PathBuf::from("file.php"),
            10,
            1,
        ));
        issues.add(Issue::error(
            "test",
            "Error 2",
            PathBuf::from("file.php"),
            20,
            1,
        ));

        let filtered = baseline.filter(issues);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.issues()[0].message, "Error 2");
    }

    #[test]
    fn test_to_neon_format() {
        let baseline = Baseline {
            entries: vec![BaselineEntry::new(
                "#^Test error$#".to_string(),
                1,
                "test.php".to_string(),
                Some("test.id".to_string()),
            )],
        };

        let neon = baseline.to_neon();
        assert!(neon.contains("parameters:"));
        assert!(neon.contains("ignoreErrors:"));
        assert!(neon.contains("message: '#^Test error$#'"));
        assert!(neon.contains("identifier: test.id"));
        assert!(neon.contains("count: 1"));
        assert!(neon.contains("path: test.php"));
    }

    #[test]
    fn test_escape_regex() {
        assert_eq!(
            escape_regex("foo().bar[]"),
            "foo\\(\\)\\.bar\\[\\]"
        );
    }
}
