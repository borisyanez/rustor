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
        let mut normalized_pattern = self.path.replace('\\', "/");
        let normalized_path = file_path.replace('\\', "/");

        // Strip leading ../ from pattern (relative path from baselines folder)
        while normalized_pattern.starts_with("../") {
            normalized_pattern = normalized_pattern[3..].to_string();
        }

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

    /// Generate a baseline from issues, merging with an existing baseline
    ///
    /// If an existing baseline is provided:
    /// - Preserves all existing entries
    /// - Adds new entries for issues not already covered
    /// - Useful for incrementally building a baseline
    pub fn generate_with_existing(existing: Option<&Baseline>, issues: &IssueCollection) -> Self {
        let new_baseline = Self::generate(issues);

        match existing {
            Some(existing) => {
                // Start with existing entries
                let mut merged_entries = existing.entries.clone();

                // Add new entries that don't already exist
                for new_entry in new_baseline.entries {
                    // Check if entry already exists by comparing:
                    // 1. Same file path
                    // 2. Same message pattern (either exact match or regex match)
                    let already_exists = existing.entries.iter().any(|e| {
                        if e.path != new_entry.path {
                            return false;
                        }
                        // Compare messages directly (both are regex patterns like #^...$#)
                        if e.message == new_entry.message {
                            return true;
                        }
                        // Also check if identifiers match (for cross-tool compatibility)
                        if let (Some(existing_id), Some(new_id)) = (&e.identifier, &new_entry.identifier) {
                            if existing_id == new_id {
                                return true;
                            }
                        }
                        false
                    });

                    if !already_exists {
                        merged_entries.push(new_entry);
                    }
                }

                // Sort for consistent output
                merged_entries.sort_by(|a, b| {
                    a.path.cmp(&b.path).then_with(|| a.message.cmp(&b.message))
                });

                Baseline { entries: merged_entries }
            }
            None => new_baseline,
        }
    }

    /// Merge another baseline into this one
    ///
    /// Adds entries from `other` that don't already exist in `self`.
    pub fn merge(&mut self, other: &Baseline) {
        for entry in &other.entries {
            let already_exists = self.entries.iter().any(|e| {
                e.path == entry.path && e.message == entry.message
            });

            if !already_exists {
                self.entries.push(entry.clone());
            }
        }

        // Sort for consistent output
        self.entries.sort_by(|a, b| {
            a.path.cmp(&b.path).then_with(|| a.message.cmp(&b.message))
        });
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
            // Use multi-line string format if message contains newlines,
            // otherwise use single-quoted string
            if entry.message.contains('\n') {
                // Multi-line NEON string format: '''...'''
                // Escape single quotes within the message
                let escaped = entry.message.replace('\'', "''");
                output.push_str("\t\t\tmessage: '''\n");
                for line in escaped.lines() {
                    output.push_str(&format!("\t\t\t\t{}\n", line));
                }
                output.push_str("\t\t\t'''\n");
            } else {
                let escaped_message = escape_neon_string(&entry.message);
                output.push_str(&format!("\t\t\tmessage: '{}'\n", escaped_message));
            }
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
        self.filter_with_options(issues, false)
    }

    /// Filter issues against the baseline with options
    /// If ignore_counts is true, baseline entries match unlimited times (not just `count` times)
    pub fn filter_with_options(&self, issues: IssueCollection, ignore_counts: bool) -> IssueCollection {
        let debug = std::env::var("RUSTOR_DEBUG").is_ok();
        if debug {
            eprintln!(">>> BASELINE FILTER START: {} baseline entries, {} issues to filter (ignore_counts={})",
                self.entries.len(), issues.len(), ignore_counts);
        }

        // Track remaining counts for each entry (only used if !ignore_counts)
        let mut remaining_counts: Vec<usize> = self.entries.iter().map(|e| e.count).collect();
        let mut filtered = IssueCollection::new();

        for issue in issues.into_issues() {
            let file_path = issue.file.display().to_string();
            let mut matched = false;
            let mut match_reason = "";

            // Try to match against baseline entries
            for (i, entry) in self.entries.iter().enumerate() {
                if !ignore_counts && remaining_counts[i] == 0 {
                    continue;
                }

                // Check path match
                let path_matches = entry.matches_path(&file_path);
                if !path_matches {
                    continue;
                }

                // Strategy 1: Exact message match (with optional identifier)
                let message_matches = entry.matches_message(&issue.message);
                let id_matches = entry.matches_identifier(issue.identifier.as_deref());

                // Debug: log when checking entries that might match apiDownPayment
                if debug && entry.message.contains("apiDownPayment") && issue.message.contains("apiDownPayment") {
                    eprintln!("[baseline] APIDOWNPAYMENT entry {} for {}:{} (remaining_count={})",
                        i, file_path, issue.line, remaining_counts[i]);
                    eprintln!("  Entry: message=\"{}\"", entry.message);
                    eprintln!("  Issue: message=\"{}\"", issue.message);
                    eprintln!("  Entry id={:?}, Issue id={:?}", entry.identifier, issue.identifier);
                    eprintln!("  path_matches={}, message_matches={}, id_matches={}",
                        path_matches, message_matches, id_matches);
                    eprintln!("  WILL FILTER: {}", message_matches && id_matches && remaining_counts[i] > 0);
                }

                if message_matches && id_matches {
                    if !ignore_counts {
                        remaining_counts[i] -= 1;
                    }
                    matched = true;
                    match_reason = "message+identifier";
                    break;
                }

                // Strategy 2: Identifier-only match (for cross-tool compatibility)
                // PHPStan and rustor may have different message wording but same identifiers
                if let (Some(entry_id), Some(issue_id)) = (&entry.identifier, &issue.identifier) {
                    if entry_id == issue_id {
                        if !ignore_counts {
                            remaining_counts[i] -= 1;
                        }
                        matched = true;
                        match_reason = "identifier-only";
                        break;
                    }
                }
            }

            if matched {
                if std::env::var("RUSTOR_DEBUG").is_ok() {
                    eprintln!(">>> BASELINE FILTERED ({}): {}:{} - {}",
                        match_reason,
                        file_path,
                        issue.line,
                        &issue.message[..issue.message.len().min(60)]
                    );
                }
                logging::log(&format!(
                    "BASELINE FILTERED ({}): {}:{} - {}",
                    match_reason,
                    file_path,
                    issue.line,
                    &issue.message[..issue.message.len().min(60)]
                ));
            } else {
                filtered.add(issue);
            }
        }

        if std::env::var("RUSTOR_DEBUG").is_ok() {
            eprintln!(">>> BASELINE FILTER: {} issues in, {} issues out",
                self.entries.len(), filtered.len());
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
    // In NEON single-quoted strings:
    // - Single quotes need escaping as ''
    // - Newlines are not allowed (replace with space)
    // - Tabs should be replaced with spaces
    s.replace('\'', "''")
        .replace('\n', " ")
        .replace('\r', "")
        .replace('\t', " ")
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

    #[test]
    fn test_generate_with_existing() {
        // Create an existing baseline with one entry
        let existing = Baseline {
            entries: vec![BaselineEntry::new(
                "#^Existing error$#".to_string(),
                1,
                "existing.php".to_string(),
                Some("existing.id".to_string()),
            )],
        };

        // Create new issues - one that matches existing, one that's new
        let mut issues = IssueCollection::new();
        issues.add(Issue::error(
            "new.id",
            "New error",
            PathBuf::from("new.php"),
            10,
            1,
        ).with_identifier("new.id"));

        // Generate with existing baseline
        let merged = Baseline::generate_with_existing(Some(&existing), &issues);

        // Should have both entries
        assert_eq!(merged.entries.len(), 2);

        // Existing entry should be preserved
        assert!(merged.entries.iter().any(|e| e.path == "existing.php"));

        // New entry should be added
        assert!(merged.entries.iter().any(|e| e.path == "new.php"));
    }

    #[test]
    fn test_generate_with_existing_no_duplicates() {
        // Create an existing baseline
        let existing = Baseline {
            entries: vec![BaselineEntry::new(
                "#^Variable \\$foo might not be defined\\.$#".to_string(),
                1,
                "test.php".to_string(),
                Some("variable.undefined".to_string()),
            )],
        };

        // Create issue with same message
        let mut issues = IssueCollection::new();
        issues.add(Issue::error(
            "variable.undefined",
            "Variable $foo might not be defined.",
            PathBuf::from("test.php"),
            10,
            1,
        ).with_identifier("variable.undefined"));

        // Generate with existing baseline
        let merged = Baseline::generate_with_existing(Some(&existing), &issues);

        // Should still have only one entry (no duplicate)
        assert_eq!(merged.entries.len(), 1);
    }

    #[test]
    fn test_merge_baseline() {
        let mut baseline1 = Baseline {
            entries: vec![BaselineEntry::new(
                "#^Error 1$#".to_string(),
                1,
                "file1.php".to_string(),
                None,
            )],
        };

        let baseline2 = Baseline {
            entries: vec![
                BaselineEntry::new(
                    "#^Error 1$#".to_string(),
                    1,
                    "file1.php".to_string(),
                    None,
                ),
                BaselineEntry::new(
                    "#^Error 2$#".to_string(),
                    1,
                    "file2.php".to_string(),
                    None,
                ),
            ],
        };

        baseline1.merge(&baseline2);

        // Should have 2 entries (1 original + 1 new, no duplicate)
        assert_eq!(baseline1.entries.len(), 2);
        assert!(baseline1.entries.iter().any(|e| e.path == "file1.php"));
        assert!(baseline1.entries.iter().any(|e| e.path == "file2.php"));
    }
}
