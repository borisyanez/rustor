//! Baseline support for gradual adoption
//!
//! Baselines allow ignoring existing errors so only new errors are reported.

use crate::issue::{Issue, IssueCollection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A baseline entry representing an ignored error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineEntry {
    pub message: String,
    pub count: usize,
    pub path: String,
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

    /// Load baseline from a NEON/JSON file
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(path)?;

        // Try JSON first
        if let Ok(baseline) = serde_json::from_str(&content) {
            return Ok(baseline);
        }

        // Parse NEON format
        Self::parse_neon(&content)
    }

    /// Parse NEON baseline format
    fn parse_neon(content: &str) -> Result<Self, std::io::Error> {
        let mut entries = Vec::new();

        // Simple NEON parsing for baseline format
        // Format:
        // parameters:
        //     ignoreErrors:
        //         -
        //             message: "#message#"
        //             count: N
        //             path: file.php

        let mut current_message = None;
        let mut current_count = None;
        let mut current_path = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("message:") {
                let msg = trimmed.trim_start_matches("message:").trim();
                let msg = msg.trim_matches(|c| c == '"' || c == '\'');
                current_message = Some(msg.to_string());
            } else if trimmed.starts_with("count:") {
                let count_str = trimmed.trim_start_matches("count:").trim();
                current_count = count_str.parse().ok();
            } else if trimmed.starts_with("path:") {
                let path = trimmed.trim_start_matches("path:").trim();
                current_path = Some(path.to_string());
            }

            // If we have all three, create an entry
            if current_message.is_some() && current_count.is_some() && current_path.is_some() {
                entries.push(BaselineEntry {
                    message: current_message.take().unwrap(),
                    count: current_count.take().unwrap(),
                    path: current_path.take().unwrap(),
                });
            }
        }

        Ok(Baseline { entries })
    }

    /// Generate a baseline from a collection of issues
    pub fn generate(issues: &IssueCollection) -> Self {
        let mut grouped: HashMap<(String, String), usize> = HashMap::new();

        for issue in issues.issues() {
            let key = (issue.file.display().to_string(), issue.message.clone());
            *grouped.entry(key).or_insert(0) += 1;
        }

        let entries = grouped
            .into_iter()
            .map(|((path, message), count)| BaselineEntry {
                message,
                count,
                path,
            })
            .collect();

        Baseline { entries }
    }

    /// Save baseline to a file
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let content = if extension == "json" {
            serde_json::to_string_pretty(self)?
        } else {
            self.to_neon()
        };

        fs::write(path, content)
    }

    /// Convert to NEON format
    fn to_neon(&self) -> String {
        let mut output = String::from("parameters:\n    ignoreErrors:\n");

        for entry in &self.entries {
            output.push_str("        -\n");
            output.push_str(&format!("            message: \"{}\"\n", escape_neon(&entry.message)));
            output.push_str(&format!("            count: {}\n", entry.count));
            output.push_str(&format!("            path: {}\n", entry.path));
        }

        output
    }

    /// Filter issues against the baseline
    pub fn filter(&self, issues: IssueCollection) -> IssueCollection {
        let mut remaining_counts: HashMap<(String, String), usize> = self
            .entries
            .iter()
            .map(|e| ((e.path.clone(), e.message.clone()), e.count))
            .collect();

        let mut filtered = IssueCollection::new();

        for issue in issues.into_issues() {
            let key = (issue.file.display().to_string(), issue.message.clone());

            if let Some(count) = remaining_counts.get_mut(&key) {
                if *count > 0 {
                    *count -= 1;
                    continue; // Skip this issue
                }
            }

            // Also check fuzzy matches (same message, any path)
            let fuzzy_key = (String::new(), issue.message.clone());
            let mut matched = false;
            for ((path, msg), count) in remaining_counts.iter_mut() {
                if path.is_empty() && msg == &issue.message && *count > 0 {
                    *count -= 1;
                    matched = true;
                    break;
                }
            }

            if !matched {
                filtered.add(issue);
            }
        }

        filtered
    }
}

/// Escape special characters for NEON string
fn escape_neon(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::Issue;
    use std::path::PathBuf;

    #[test]
    fn test_generate_baseline() {
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
            "Error 1",
            PathBuf::from("file.php"),
            20,
            1,
        ));

        let baseline = Baseline::generate(&issues);
        assert_eq!(baseline.entries.len(), 1);
        assert_eq!(baseline.entries[0].count, 2);
    }

    #[test]
    fn test_filter_baseline() {
        let baseline = Baseline {
            entries: vec![BaselineEntry {
                message: "Error 1".to_string(),
                count: 1,
                path: "file.php".to_string(),
            }],
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
}
