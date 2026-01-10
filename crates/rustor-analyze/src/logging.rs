//! Logging module for rustor-analyze
//!
//! Provides detailed logging of configuration parsing, includes processing,
//! and error filtering for debugging and verification purposes.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Global logger instance
static LOGGER: Mutex<Option<AnalyzeLogger>> = Mutex::new(None);

/// Logger for analyze operations
pub struct AnalyzeLogger {
    file: File,
    path: PathBuf,
}

impl AnalyzeLogger {
    /// Create a new logger writing to the specified path
    pub fn new(log_path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path)?;

        Ok(Self {
            file,
            path: log_path.to_path_buf(),
        })
    }

    /// Write a log message
    pub fn log(&mut self, message: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(self.file, "[{}] {}", timestamp, message);
        let _ = self.file.flush();
    }

    /// Log a section header
    pub fn section(&mut self, title: &str) {
        let separator = "=".repeat(60);
        self.log(&separator);
        self.log(title);
        self.log(&separator);
    }

    /// Log a subsection
    pub fn subsection(&mut self, title: &str) {
        let separator = "-".repeat(40);
        self.log(&separator);
        self.log(title);
        self.log(&separator);
    }
}

/// Initialize the global logger
pub fn init_logger(log_path: Option<&Path>) -> std::io::Result<PathBuf> {
    let path = log_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            PathBuf::from(format!("/tmp/rustor-analyze-{}.log", timestamp))
        });

    let logger = AnalyzeLogger::new(&path)?;

    if let Ok(mut guard) = LOGGER.lock() {
        *guard = Some(logger);
    }

    Ok(path)
}

/// Log a message to the global logger
pub fn log(message: &str) {
    if let Ok(mut guard) = LOGGER.lock() {
        if let Some(ref mut logger) = *guard {
            logger.log(message);
        }
    }
}

/// Log a section header
pub fn section(title: &str) {
    if let Ok(mut guard) = LOGGER.lock() {
        if let Some(ref mut logger) = *guard {
            logger.section(title);
        }
    }
}

/// Log a subsection
pub fn subsection(title: &str) {
    if let Ok(mut guard) = LOGGER.lock() {
        if let Some(ref mut logger) = *guard {
            logger.subsection(title);
        }
    }
}

/// Check if logging is enabled
pub fn is_enabled() -> bool {
    if let Ok(guard) = LOGGER.lock() {
        guard.is_some()
    } else {
        false
    }
}

/// Log configuration loading
pub fn log_config_load(path: &Path) {
    section("CONFIGURATION LOADING");
    log(&format!("Loading config from: {}", path.display()));
}

/// Log include file processing
pub fn log_include_start(include_path: &Path, base_dir: &Path) {
    subsection("PROCESSING INCLUDE");
    log(&format!("Include path: {}", include_path.display()));
    log(&format!("Base directory: {}", base_dir.display()));
}

/// Log include file result
pub fn log_include_result(include_path: &Path, success: bool, error: Option<&str>) {
    if success {
        log(&format!("Successfully loaded: {}", include_path.display()));
    } else {
        log(&format!("FAILED to load: {}", include_path.display()));
        if let Some(err) = error {
            log(&format!("  Error: {}", err));
        }
    }
}

/// Log parameters being merged
pub fn log_parameters_merge(source: &str, param_name: &str, value: &str) {
    log(&format!("[{}] {} = {}", source, param_name, value));
}

/// Log ignore errors being loaded
pub fn log_ignore_errors(source: &str, count: usize) {
    log(&format!("[{}] Loaded {} ignore error patterns", source, count));
}

/// Log individual ignore error pattern
pub fn log_ignore_error_pattern(index: usize, message: &str, identifier: Option<&str>, path: Option<&str>) {
    let mut pattern_info = format!("  [{}] message: {}", index, message);
    if let Some(id) = identifier {
        pattern_info.push_str(&format!(", identifier: {}", id));
    }
    if let Some(p) = path {
        pattern_info.push_str(&format!(", path: {}", p));
    }
    log(&pattern_info);
}

/// Log error filtering decision
pub fn log_error_filter(file: &Path, line: usize, message: &str, identifier: Option<&str>, filtered: bool, reason: Option<&str>) {
    if filtered {
        log(&format!(
            "FILTERED: {}:{} - {}",
            file.display(),
            line,
            message
        ));
        if let Some(id) = identifier {
            log(&format!("  Identifier: {}", id));
        }
        if let Some(r) = reason {
            log(&format!("  Matched by: {}", r));
        }
    }
}

/// Log summary of configuration
pub fn log_config_summary(
    level: u8,
    paths_count: usize,
    exclude_count: usize,
    ignore_errors_count: usize,
    includes_count: usize,
) {
    section("CONFIGURATION SUMMARY");
    log(&format!("Analysis level: {}", level));
    log(&format!("Paths to analyze: {}", paths_count));
    log(&format!("Exclude patterns: {}", exclude_count));
    log(&format!("Ignore error patterns: {}", ignore_errors_count));
    log(&format!("Included config files: {}", includes_count));
}

/// Log analysis start
pub fn log_analysis_start(files_count: usize) {
    section("ANALYSIS START");
    log(&format!("Analyzing {} files", files_count));
}

/// Log analysis complete
pub fn log_analysis_complete(total_errors: usize, filtered_errors: usize) {
    section("ANALYSIS COMPLETE");
    log(&format!("Total errors found: {}", total_errors));
    log(&format!("Errors filtered by ignore patterns: {}", filtered_errors));
    log(&format!("Errors reported: {}", total_errors - filtered_errors));
}
