//! Output formatting for rustor
//!
//! Supports text (colored terminal) and JSON output formats.

use colored::*;
use serde::Serialize;
use std::path::Path;

/// Output format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Diff,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<OutputFormat> {
        match s.to_lowercase().as_str() {
            "text" => Some(OutputFormat::Text),
            "json" => Some(OutputFormat::Json),
            "diff" => Some(OutputFormat::Diff),
            _ => None,
        }
    }
}

/// Information about a single edit
#[derive(Debug, Clone, Serialize)]
pub struct EditInfo {
    pub rule: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

/// Result of processing a single file
#[derive(Debug, Clone, Serialize)]
pub struct FileResult {
    pub path: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub edits: Vec<EditInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FileResult {
    pub fn success(path: &Path, edits: Vec<EditInfo>) -> Self {
        Self {
            path: path.display().to_string(),
            edits,
            error: None,
        }
    }

    pub fn error(path: &Path, error: String) -> Self {
        Self {
            path: path.display().to_string(),
            edits: Vec::new(),
            error: Some(error),
        }
    }

    #[allow(dead_code)]
    pub fn has_changes(&self) -> bool {
        !self.edits.is_empty()
    }

    #[allow(dead_code)]
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
}

/// Summary statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct Summary {
    pub files_processed: usize,
    pub files_with_changes: usize,
    pub total_edits: usize,
    pub errors: usize,
}

/// Full JSON output structure
#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub version: String,
    pub summary: Summary,
    pub files: Vec<FileResult>,
}

/// Reporter for accumulating and outputting results
pub struct Reporter {
    format: OutputFormat,
    verbose: bool,
    results: Vec<FileResult>,
    summary: Summary,
}

impl Reporter {
    pub fn new(format: OutputFormat, verbose: bool) -> Self {
        Self {
            format,
            verbose,
            results: Vec::new(),
            summary: Summary::default(),
        }
    }

    /// Report a file with changes (in check mode - showing what would change)
    pub fn report_check(&mut self, path: &Path, edits: Vec<EditInfo>, old_source: &str, new_source: &str) {
        self.summary.files_processed += 1;

        if edits.is_empty() {
            if self.verbose && self.format == OutputFormat::Text {
                println!("{}: No changes needed", path.display());
            }
            self.results.push(FileResult::success(path, vec![]));
            return;
        }

        self.summary.files_with_changes += 1;
        self.summary.total_edits += edits.len();

        match self.format {
            OutputFormat::Text => {
                println!("{}", path.display().to_string().bold());
                print_diff(old_source, new_source);
                println!();
                for edit in &edits {
                    println!("  {} {}", "->".green(), edit.message);
                }
                println!();
            }
            OutputFormat::Diff => {
                print_unified_diff(path, old_source, new_source);
            }
            OutputFormat::Json => {
                // JSON output is handled in finish()
            }
        }

        self.results.push(FileResult::success(path, edits));
    }

    /// Report a file after applying fixes
    pub fn report_fix(&mut self, path: &Path, edits: Vec<EditInfo>) {
        self.summary.files_processed += 1;

        if edits.is_empty() {
            if self.verbose && self.format == OutputFormat::Text {
                println!("{}: No changes needed", path.display());
            }
            self.results.push(FileResult::success(path, vec![]));
            return;
        }

        self.summary.files_with_changes += 1;
        self.summary.total_edits += edits.len();

        if self.format == OutputFormat::Text {
            println!("{}", path.display().to_string().bold());
            println!(
                "  {} Applied {} change(s)",
                "OK".green(),
                edits.len()
            );
            println!();
        }

        self.results.push(FileResult::success(path, edits));
    }

    /// Report a file that was skipped (no changes, not verbose)
    pub fn report_skipped(&mut self, path: &Path) {
        self.summary.files_processed += 1;
        if self.verbose && self.format == OutputFormat::Text {
            println!("{}: No changes needed", path.display());
        }
        self.results.push(FileResult::success(path, vec![]));
    }

    /// Report a file with cached results (has edits but no details available)
    pub fn report_cached(&mut self, path: &Path, edit_count: usize) {
        self.summary.files_processed += 1;
        self.summary.files_with_changes += 1;
        self.summary.total_edits += edit_count;

        if self.format == OutputFormat::Text {
            println!("{}", path.display().to_string().bold());
            println!(
                "  {} {} change(s) (cached)",
                "!".yellow(),
                edit_count
            );
            println!();
        }

        // For JSON output, we don't have detailed edit info
        self.results.push(FileResult::success(path, vec![]));
    }

    /// Report an error processing a file
    pub fn report_error(&mut self, path: &Path, error: &str) {
        self.summary.files_processed += 1;
        self.summary.errors += 1;

        if self.format == OutputFormat::Text {
            eprintln!(
                "{}: {} - {}",
                "Warning".yellow(),
                path.display(),
                error
            );
        }

        self.results.push(FileResult::error(path, error.to_string()));
    }

    /// Print final summary/output
    pub fn finish(self, check_mode: bool) {
        match self.format {
            OutputFormat::Text => {
                println!();
                println!("{}", "Summary".bold().underline());
                println!("  Files processed: {}", self.summary.files_processed);
                println!("  Files with changes: {}", self.summary.files_with_changes);
                println!("  Total edits: {}", self.summary.total_edits);
                if self.summary.errors > 0 {
                    println!("  Errors: {}", self.summary.errors);
                }

                if check_mode && self.summary.total_edits > 0 {
                    println!();
                    println!("{}", "Run with --fix to apply changes".yellow());
                }
            }
            OutputFormat::Json => {
                let output = JsonOutput {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    summary: self.summary,
                    files: self.results,
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            OutputFormat::Diff => {
                // Diff format outputs each file's diff as it's processed
                // No summary needed for patch-compatible output
            }
        }
    }

    /// Get summary for exit code determination
    pub fn summary(&self) -> &Summary {
        &self.summary
    }
}

/// Print a colored diff between old and new content
fn print_diff(old: &str, new: &str) {
    for diff_result in diff::lines(old, new) {
        match diff_result {
            diff::Result::Left(l) => {
                println!("  {}", format!("- {}", l).red());
            }
            diff::Result::Right(r) => {
                println!("  {}", format!("+ {}", r).green());
            }
            diff::Result::Both(_, _) => {
                // Skip unchanged lines for cleaner output
            }
        }
    }
}

/// Print unified diff format (standard diff -u compatible)
fn print_unified_diff(path: &Path, old: &str, new: &str) {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old, new);
    let path_str = path.display().to_string();

    // Print unified diff header
    println!("--- a/{}", path_str);
    println!("+++ b/{}", path_str);

    // Print hunks with context
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        println!("{}", hunk.header());
        for change in hunk.iter_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            print!("{}{}", sign, change);
            if change.missing_newline() {
                println!();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("text"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::from_str("TEXT"), Some(OutputFormat::Text));
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("JSON"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("diff"), Some(OutputFormat::Diff));
        assert_eq!(OutputFormat::from_str("DIFF"), Some(OutputFormat::Diff));
        assert_eq!(OutputFormat::from_str("xml"), None);
    }

    #[test]
    fn test_file_result_success() {
        let result = FileResult::success(Path::new("test.php"), vec![]);
        assert!(!result.has_changes());
        assert!(!result.has_error());
    }

    #[test]
    fn test_file_result_with_edits() {
        let edits = vec![EditInfo {
            rule: "array_push".to_string(),
            line: 10,
            column: 5,
            message: "test".to_string(),
        }];
        let result = FileResult::success(Path::new("test.php"), edits);
        assert!(result.has_changes());
        assert!(!result.has_error());
    }

    #[test]
    fn test_file_result_error() {
        let result = FileResult::error(Path::new("test.php"), "parse error".to_string());
        assert!(!result.has_changes());
        assert!(result.has_error());
    }

    #[test]
    fn test_json_serialization() {
        let output = JsonOutput {
            version: "0.2.0".to_string(),
            summary: Summary {
                files_processed: 10,
                files_with_changes: 3,
                total_edits: 7,
                errors: 0,
            },
            files: vec![FileResult::success(
                Path::new("test.php"),
                vec![EditInfo {
                    rule: "array_push".to_string(),
                    line: 15,
                    column: 5,
                    message: "Convert array_push".to_string(),
                }],
            )],
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"version\":\"0.2.0\""));
        assert!(json.contains("\"files_processed\":10"));
        assert!(json.contains("\"rule\":\"array_push\""));
    }
}
