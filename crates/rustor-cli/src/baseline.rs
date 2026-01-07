//! Baseline support for rustor
//!
//! Allows gradual adoption of rustor by tracking existing issues
//! in a baseline file and only reporting new issues.
//!
//! Usage:
//! ```bash
//! # Generate baseline for existing project
//! rustor src/ --generate-baseline > .rustor-baseline.json
//!
//! # Run with baseline (only new issues reported)
//! rustor src/ --baseline .rustor-baseline.json
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use anyhow::{Context, Result};

use crate::output::EditInfo;

/// Baseline file format version
const BASELINE_VERSION: u32 = 1;

/// A single issue in the baseline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BaselineIssue {
    /// Relative file path
    pub file: String,
    /// Line number (1-based)
    pub line: usize,
    /// Rule that triggered the issue
    pub rule: String,
    /// Content hash for fuzzy matching (first 20 chars of the line)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// The baseline file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    /// Version of the baseline format
    pub version: u32,
    /// When the baseline was generated (ISO 8601)
    pub generated: String,
    /// List of known issues
    pub issues: Vec<BaselineIssue>,
}

impl Baseline {
    /// Create a new empty baseline
    pub fn new() -> Self {
        Self {
            version: BASELINE_VERSION,
            generated: chrono_lite_now(),
            issues: Vec::new(),
        }
    }

    /// Load a baseline from a file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read baseline file: {}", path.display()))?;

        let baseline: Baseline = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse baseline file: {}", path.display()))?;

        // Check version compatibility
        if baseline.version > BASELINE_VERSION {
            anyhow::bail!(
                "Baseline file version {} is newer than supported version {}",
                baseline.version,
                BASELINE_VERSION
            );
        }

        Ok(baseline)
    }

    /// Generate a baseline from processing results
    pub fn generate(files: &[(String, Vec<EditInfo>, Option<String>)]) -> Self {
        let mut issues = Vec::new();

        for (path, edits, source) in files {
            let lines: Vec<&str> = source.as_ref().map(|s| s.lines().collect()).unwrap_or_default();

            for edit in edits {
                let context = if edit.line > 0 && edit.line <= lines.len() {
                    let line_content = lines[edit.line - 1];
                    // Take first 40 chars as context for fuzzy matching
                    Some(line_content.trim().chars().take(40).collect())
                } else {
                    None
                };

                issues.push(BaselineIssue {
                    file: path.clone(),
                    line: edit.line,
                    rule: edit.rule.clone(),
                    context,
                });
            }
        }

        Self {
            version: BASELINE_VERSION,
            generated: chrono_lite_now(),
            issues,
        }
    }

    /// Filter out issues that are in the baseline
    pub fn filter_edits(&self, path: &str, edits: Vec<EditInfo>, source: &str) -> Vec<EditInfo> {
        let lines: Vec<&str> = source.lines().collect();

        // Build a set of baseline issues for this file
        let baseline_issues: HashSet<_> = self.issues
            .iter()
            .filter(|issue| issue.file == path)
            .cloned()
            .collect();

        edits.into_iter().filter(|edit| {
            let context = if edit.line > 0 && edit.line <= lines.len() {
                let line_content = lines[edit.line - 1];
                Some(line_content.trim().chars().take(40).collect::<String>())
            } else {
                None
            };

            // Create issue to compare
            let current_issue = BaselineIssue {
                file: path.to_string(),
                line: edit.line,
                rule: edit.rule.clone(),
                context: context.clone(),
            };

            // Check exact match first
            if baseline_issues.contains(&current_issue) {
                return false;
            }

            // Fuzzy match: same file, rule, and context (line may have shifted)
            let fuzzy_match = baseline_issues.iter().any(|base| {
                base.file == path
                    && base.rule == edit.rule
                    && base.context.is_some()
                    && base.context == context
            });

            !fuzzy_match
        }).collect()
    }

    /// Serialize baseline to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .context("Failed to serialize baseline")
    }

    /// Get the number of issues
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Check if baseline is empty
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }
}

impl Default for Baseline {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple ISO 8601 timestamp without external dependencies
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();

    // Simple calculation (not accounting for leap years/seconds precisely)
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Days since Unix epoch to year/month/day (simplified)
    let mut year = 1970;
    let mut remaining_days = days as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let month_days: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days_in_month in month_days {
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_new() {
        let baseline = Baseline::new();
        assert_eq!(baseline.version, BASELINE_VERSION);
        assert!(baseline.issues.is_empty());
    }

    #[test]
    fn test_baseline_generate() {
        let files = vec![(
            "test.php".to_string(),
            vec![
                EditInfo {
                    rule: "is_null".to_string(),
                    line: 5,
                    column: 1,
                    message: "test".to_string(),
                },
            ],
            Some("<?php\nline2\nline3\nline4\n$x = is_null($y);".to_string()),
        )];

        let baseline = Baseline::generate(&files);
        assert_eq!(baseline.issues.len(), 1);
        assert_eq!(baseline.issues[0].file, "test.php");
        assert_eq!(baseline.issues[0].rule, "is_null");
        assert_eq!(baseline.issues[0].line, 5);
    }

    #[test]
    fn test_baseline_filter_exact() {
        let mut baseline = Baseline::new();
        baseline.issues.push(BaselineIssue {
            file: "test.php".to_string(),
            line: 5,
            rule: "is_null".to_string(),
            context: Some("$x = is_null($y);".to_string()),
        });

        let edits = vec![
            EditInfo {
                rule: "is_null".to_string(),
                line: 5,
                column: 1,
                message: "test".to_string(),
            },
            EditInfo {
                rule: "array_push".to_string(),
                line: 10,
                column: 1,
                message: "test2".to_string(),
            },
        ];

        let source = "line1\nline2\nline3\nline4\n$x = is_null($y);\nline6\nline7\nline8\nline9\narray_push($arr, $v);";
        let filtered = baseline.filter_edits("test.php", edits, source);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rule, "array_push");
    }

    #[test]
    fn test_baseline_filter_fuzzy() {
        let mut baseline = Baseline::new();
        baseline.issues.push(BaselineIssue {
            file: "test.php".to_string(),
            line: 5, // Original line
            rule: "is_null".to_string(),
            context: Some("$x = is_null($y);".to_string()),
        });

        // Same content but on line 7 (line shifted due to new code)
        let edits = vec![EditInfo {
            rule: "is_null".to_string(),
            line: 7,
            column: 1,
            message: "test".to_string(),
        }];

        let source = "line1\nline2\nline3\nline4\nline5\nline6\n$x = is_null($y);";
        let filtered = baseline.filter_edits("test.php", edits, source);

        // Should be filtered out due to fuzzy match on context
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_baseline_serialization() {
        let mut baseline = Baseline::new();
        baseline.issues.push(BaselineIssue {
            file: "test.php".to_string(),
            line: 5,
            rule: "is_null".to_string(),
            context: Some("test context".to_string()),
        });

        let json = baseline.to_json().unwrap();
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"file\": \"test.php\""));

        // Verify it can be parsed back
        let parsed: Baseline = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.issues.len(), 1);
    }
}
