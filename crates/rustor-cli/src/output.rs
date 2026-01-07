//! Output formatting for rustor
//!
//! Supports text (colored terminal), JSON, SARIF, HTML, and unified diff output formats.

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
    Sarif,
    Html,
    Checkstyle,
    Github,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<OutputFormat> {
        match s.to_lowercase().as_str() {
            "text" => Some(OutputFormat::Text),
            "json" => Some(OutputFormat::Json),
            "diff" => Some(OutputFormat::Diff),
            "sarif" => Some(OutputFormat::Sarif),
            "html" => Some(OutputFormat::Html),
            "checkstyle" => Some(OutputFormat::Checkstyle),
            "github" => Some(OutputFormat::Github),
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

/// Extended file info for SARIF/HTML output
#[derive(Debug, Clone)]
pub struct ExtendedFileResult {
    pub path: String,
    pub edits: Vec<EditInfo>,
    pub old_source: Option<String>,
    pub new_source: Option<String>,
    pub error: Option<String>,
}

/// Reporter for accumulating and outputting results
pub struct Reporter {
    format: OutputFormat,
    verbose: bool,
    results: Vec<FileResult>,
    extended_results: Vec<ExtendedFileResult>,
    summary: Summary,
    enabled_rules: Vec<String>,
}

impl Reporter {
    pub fn new(format: OutputFormat, verbose: bool) -> Self {
        Self {
            format,
            verbose,
            results: Vec::new(),
            extended_results: Vec::new(),
            summary: Summary::default(),
            enabled_rules: Vec::new(),
        }
    }

    /// Set the list of enabled rules (for SARIF output)
    pub fn set_enabled_rules(&mut self, rules: Vec<String>) {
        self.enabled_rules = rules;
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
            OutputFormat::Json | OutputFormat::Sarif | OutputFormat::Html | OutputFormat::Checkstyle => {
                // Output is handled in finish()
            }
            OutputFormat::Github => {
                // Output GitHub workflow commands immediately
                for edit in &edits {
                    println!(
                        "::warning file={},line={},col={}::{} ({})",
                        path.display(),
                        edit.line,
                        edit.column,
                        edit.message,
                        edit.rule
                    );
                }
            }
        }

        // Store extended result for SARIF/HTML
        self.extended_results.push(ExtendedFileResult {
            path: path.display().to_string(),
            edits: edits.clone(),
            old_source: Some(old_source.to_string()),
            new_source: Some(new_source.to_string()),
            error: None,
        });

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
            OutputFormat::Sarif => {
                let sarif = generate_sarif(&self.extended_results, &self.enabled_rules);
                println!("{}", serde_json::to_string_pretty(&sarif).unwrap());
            }
            OutputFormat::Html => {
                let html = generate_html(&self.extended_results, &self.summary);
                println!("{}", html);
            }
            OutputFormat::Checkstyle => {
                let xml = generate_checkstyle(&self.extended_results);
                println!("{}", xml);
            }
            OutputFormat::Github => {
                // GitHub annotations are printed as they're processed
                // Just print a summary as a notice
                if self.summary.files_with_changes > 0 {
                    println!(
                        "::notice::Rustor found {} edit(s) in {} file(s)",
                        self.summary.total_edits,
                        self.summary.files_with_changes
                    );
                }
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

// ==================== SARIF Output ====================

/// SARIF 2.1.0 output structure
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifOutput {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: &'static str,
    version: String,
    information_uri: &'static str,
    rules: Vec<SarifRule>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule {
    id: String,
    short_description: SarifMessage,
    help_uri: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    rule_id: String,
    level: &'static str,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixes: Option<Vec<SarifFix>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifMessage {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRegion {
    start_line: usize,
    start_column: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifFix {
    description: SarifMessage,
    artifact_changes: Vec<SarifArtifactChange>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifArtifactChange {
    artifact_location: SarifArtifactLocation,
    replacements: Vec<SarifReplacement>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifReplacement {
    deleted_region: SarifRegion,
    inserted_content: SarifContent,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifContent {
    text: String,
}

/// Generate SARIF 2.1.0 output
fn generate_sarif(results: &[ExtendedFileResult], enabled_rules: &[String]) -> SarifOutput {
    // Build rule definitions
    let rules: Vec<SarifRule> = enabled_rules
        .iter()
        .map(|rule| SarifRule {
            id: rule.clone(),
            short_description: SarifMessage {
                text: format!("Rustor rule: {}", rule),
            },
            help_uri: None,
        })
        .collect();

    // Build results
    let sarif_results: Vec<SarifResult> = results
        .iter()
        .flat_map(|file| {
            file.edits.iter().map(move |edit| SarifResult {
                rule_id: edit.rule.clone(),
                level: "warning",
                message: SarifMessage {
                    text: edit.message.clone(),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: file.path.clone(),
                        },
                        region: SarifRegion {
                            start_line: edit.line,
                            start_column: edit.column,
                        },
                    },
                }],
                fixes: None, // Could add fix suggestions here
            })
        })
        .collect();

    SarifOutput {
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        version: "2.1.0",
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "rustor",
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    information_uri: "https://github.com/your-org/rustor",
                    rules,
                },
            },
            results: sarif_results,
        }],
    }
}

// ==================== HTML Output ====================

/// Generate standalone HTML report
fn generate_html(results: &[ExtendedFileResult], summary: &Summary) -> String {
    use similar::{ChangeTag, TextDiff};

    let files_with_changes: Vec<_> = results
        .iter()
        .filter(|r| !r.edits.is_empty())
        .collect();

    let mut html = String::new();

    // HTML header with embedded CSS
    html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rustor Analysis Report</title>
    <style>
        :root {
            --bg-color: #1e1e1e;
            --text-color: #d4d4d4;
            --header-bg: #252526;
            --border-color: #3c3c3c;
            --success-color: #4ec9b0;
            --warning-color: #dcdcaa;
            --error-color: #f14c4c;
            --add-bg: #1e3a1e;
            --del-bg: #3a1e1e;
            --add-color: #89d185;
            --del-color: #f14c4c;
            --link-color: #569cd6;
        }
        * { box-sizing: border-box; }
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: var(--bg-color);
            color: var(--text-color);
            margin: 0;
            padding: 20px;
            line-height: 1.6;
        }
        .container { max-width: 1200px; margin: 0 auto; }
        h1 {
            color: var(--success-color);
            border-bottom: 2px solid var(--border-color);
            padding-bottom: 10px;
        }
        .summary {
            background: var(--header-bg);
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 30px;
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 20px;
        }
        .stat {
            text-align: center;
        }
        .stat-value {
            font-size: 2em;
            font-weight: bold;
            color: var(--success-color);
        }
        .stat-value.warning { color: var(--warning-color); }
        .stat-value.error { color: var(--error-color); }
        .stat-label {
            font-size: 0.9em;
            color: #888;
        }
        .file-section {
            background: var(--header-bg);
            border-radius: 8px;
            margin-bottom: 20px;
            overflow: hidden;
        }
        .file-header {
            background: #2d2d2d;
            padding: 12px 16px;
            cursor: pointer;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .file-header:hover { background: #333; }
        .file-path {
            font-family: 'Consolas', 'Monaco', monospace;
            color: var(--link-color);
        }
        .edit-count {
            background: var(--warning-color);
            color: #000;
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 0.85em;
            font-weight: bold;
        }
        .file-content {
            padding: 0;
            display: none;
        }
        .file-section.open .file-content { display: block; }
        .diff {
            font-family: 'Consolas', 'Monaco', monospace;
            font-size: 13px;
            overflow-x: auto;
            margin: 0;
            padding: 16px;
        }
        .diff-line {
            white-space: pre;
            padding: 2px 8px;
        }
        .diff-add {
            background: var(--add-bg);
            color: var(--add-color);
        }
        .diff-del {
            background: var(--del-bg);
            color: var(--del-color);
        }
        .messages {
            padding: 16px;
            border-top: 1px solid var(--border-color);
        }
        .message {
            padding: 6px 0;
            color: var(--warning-color);
        }
        .message::before {
            content: "→ ";
            color: var(--success-color);
        }
        .no-changes {
            text-align: center;
            padding: 40px;
            color: #888;
        }
        footer {
            text-align: center;
            padding: 20px;
            color: #666;
            font-size: 0.9em;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🦀 Rustor Analysis Report</h1>
"#);

    // Summary section
    html.push_str(&format!(r#"
        <div class="summary">
            <div class="stat">
                <div class="stat-value">{}</div>
                <div class="stat-label">Files Processed</div>
            </div>
            <div class="stat">
                <div class="stat-value warning">{}</div>
                <div class="stat-label">Files with Changes</div>
            </div>
            <div class="stat">
                <div class="stat-value warning">{}</div>
                <div class="stat-label">Total Edits</div>
            </div>
            <div class="stat">
                <div class="stat-value{}">{}</div>
                <div class="stat-label">Errors</div>
            </div>
        </div>
"#,
        summary.files_processed,
        summary.files_with_changes,
        summary.total_edits,
        if summary.errors > 0 { " error" } else { "" },
        summary.errors
    ));

    if files_with_changes.is_empty() {
        html.push_str(r#"
        <div class="no-changes">
            <p>✨ No changes needed - code looks good!</p>
        </div>
"#);
    } else {
        for file in files_with_changes {
            let file_name = Path::new(&file.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file.path.clone());

            html.push_str(&format!(r#"
        <div class="file-section open">
            <div class="file-header" onclick="this.parentElement.classList.toggle('open')">
                <span class="file-path" title="{}">{}</span>
                <span class="edit-count">{} change{}</span>
            </div>
            <div class="file-content">
                <div class="diff">
"#,
                html_escape(&file.path),
                html_escape(&file_name),
                file.edits.len(),
                if file.edits.len() == 1 { "" } else { "s" }
            ));

            // Generate diff
            if let (Some(old), Some(new)) = (&file.old_source, &file.new_source) {
                let diff = TextDiff::from_lines(old, new);
                for change in diff.iter_all_changes() {
                    let (class, prefix) = match change.tag() {
                        ChangeTag::Delete => ("diff-del", "-"),
                        ChangeTag::Insert => ("diff-add", "+"),
                        ChangeTag::Equal => ("", " "),
                    };
                    let line = change.value().trim_end_matches('\n');
                    html.push_str(&format!(
                        r#"<div class="diff-line {}">{}{}</div>
"#,
                        class,
                        prefix,
                        html_escape(line)
                    ));
                }
            }

            html.push_str(r#"                </div>
                <div class="messages">
"#);

            for edit in &file.edits {
                html.push_str(&format!(
                    r#"                    <div class="message">{}</div>
"#,
                    html_escape(&edit.message)
                ));
            }

            html.push_str(r#"                </div>
            </div>
        </div>
"#);
        }
    }

    // Footer
    html.push_str(&format!(r#"
        <footer>
            Generated by rustor v{} • <a href="https://github.com/your-org/rustor" style="color: var(--link-color)">GitHub</a>
        </footer>
    </div>
    <script>
        // Toggle file sections on click
        document.querySelectorAll('.file-header').forEach(header => {{
            header.addEventListener('click', () => {{
                header.parentElement.classList.toggle('open');
            }});
        }});
    </script>
</body>
</html>
"#, env!("CARGO_PKG_VERSION")));

    html
}

// ==================== Checkstyle XML Output ====================

/// Generate Checkstyle XML output (for CI tools like Jenkins)
fn generate_checkstyle(results: &[ExtendedFileResult]) -> String {
    let mut xml = String::new();

    // XML header
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<checkstyle version="4.3">
"#);

    // Group edits by file
    for file in results {
        if file.edits.is_empty() {
            continue;
        }

        xml.push_str(&format!(r#"  <file name="{}">
"#, xml_escape(&file.path)));

        for edit in &file.edits {
            xml.push_str(&format!(
                r#"    <error line="{}" column="{}" severity="warning" message="{}" source="rustor.{}"/>
"#,
                edit.line,
                edit.column,
                xml_escape(&edit.message),
                edit.rule
            ));
        }

        xml.push_str("  </file>\n");
    }

    xml.push_str("</checkstyle>\n");
    xml
}

/// Escape XML special characters
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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
        assert_eq!(OutputFormat::from_str("sarif"), Some(OutputFormat::Sarif));
        assert_eq!(OutputFormat::from_str("SARIF"), Some(OutputFormat::Sarif));
        assert_eq!(OutputFormat::from_str("html"), Some(OutputFormat::Html));
        assert_eq!(OutputFormat::from_str("HTML"), Some(OutputFormat::Html));
        assert_eq!(OutputFormat::from_str("checkstyle"), Some(OutputFormat::Checkstyle));
        assert_eq!(OutputFormat::from_str("CHECKSTYLE"), Some(OutputFormat::Checkstyle));
        assert_eq!(OutputFormat::from_str("github"), Some(OutputFormat::Github));
        assert_eq!(OutputFormat::from_str("GITHUB"), Some(OutputFormat::Github));
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
