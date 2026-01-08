//! rustor-analyze: PHPStan-compatible static analysis for PHP
//!
//! This crate provides static analysis capabilities compatible with PHPStan,
//! including:
//!
//! - NEON configuration file parsing
//! - PHPStan-compatible analysis levels (0-9)
//! - Multiple output formats (raw, json, table, github)
//! - Baseline support for gradual adoption
//!
//! # Example
//!
//! ```no_run
//! use rustor_analyze::{Analyzer, config::PhpStanConfig, output::OutputFormat};
//! use std::path::Path;
//!
//! // Load configuration
//! let config = PhpStanConfig::load(Path::new("phpstan.neon")).unwrap();
//!
//! // Create analyzer
//! let analyzer = Analyzer::new(config);
//!
//! // Run analysis
//! let issues = analyzer.analyze_paths(&[Path::new("src/")]).unwrap();
//!
//! // Format output
//! let output = rustor_analyze::output::format_issues(&issues, OutputFormat::Table);
//! println!("{}", output);
//! ```

pub mod baseline;
pub mod checks;
pub mod config;
pub mod issue;
pub mod output;

use checks::{CheckContext, CheckRegistry, PHP_BUILTIN_CLASSES, PHP_BUILTIN_FUNCTIONS};
use config::PhpStanConfig;
use issue::IssueCollection;
use mago_database::file::FileId;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Main analyzer that runs static analysis checks
pub struct Analyzer {
    config: PhpStanConfig,
    registry: CheckRegistry,
}

impl Analyzer {
    /// Create a new analyzer with the given configuration
    pub fn new(config: PhpStanConfig) -> Self {
        let registry = CheckRegistry::with_builtin_checks();
        Self { config, registry }
    }

    /// Create analyzer with default configuration
    pub fn with_defaults() -> Self {
        Self::new(PhpStanConfig::default())
    }

    /// Set the analysis level
    pub fn set_level(&mut self, level: config::Level) {
        self.config.level = level;
    }

    /// Get the current configuration
    pub fn config(&self) -> &PhpStanConfig {
        &self.config
    }

    /// Analyze a single file
    pub fn analyze_file(&self, path: &Path) -> Result<IssueCollection, AnalyzeError> {
        let source = fs::read_to_string(path)?;
        self.analyze_source(path, &source)
    }

    /// Analyze source code with a given path
    pub fn analyze_source(&self, path: &Path, source: &str) -> Result<IssueCollection, AnalyzeError> {
        // Parse the PHP file using bumpalo arena
        let arena = bumpalo::Bump::new();
        let file_id = FileId::new(path.to_string_lossy().as_ref());
        let (program, parse_error) = mago_syntax::parser::parse_file_content(&arena, file_id, source);

        let mut issues = IssueCollection::new();

        // Report parse errors
        if let Some(error) = parse_error {
            issues.add(issue::Issue::error(
                "parse.error",
                error.to_string(),
                path.to_path_buf(),
                1, // TODO: Get line from error
                1,
            ));
        }

        // Create check context
        let ctx = CheckContext {
            file_path: path,
            source,
            config: &self.config,
            builtin_functions: PHP_BUILTIN_FUNCTIONS,
            builtin_classes: PHP_BUILTIN_CLASSES,
        };

        // Run checks for the configured level
        let checks = self.registry.checks_for_level(self.config.level.as_u8());
        for check in checks {
            let check_issues = check.check(&program, &ctx);
            for issue in check_issues {
                // Filter ignored errors
                if !self.config.should_ignore_error(
                    &issue.message,
                    &issue.file,
                    issue.identifier.as_deref(),
                ) {
                    issues.add(issue);
                }
            }
        }

        Ok(issues)
    }

    /// Analyze multiple paths (files or directories)
    pub fn analyze_paths(&self, paths: &[&Path]) -> Result<IssueCollection, AnalyzeError> {
        // Collect all PHP files
        let mut files: Vec<_> = Vec::new();

        for path in paths {
            if path.is_file() {
                files.push(path.to_path_buf());
            } else if path.is_dir() {
                for entry in WalkDir::new(path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let entry_path = entry.path();
                    if entry_path.is_file()
                        && entry_path.extension().map(|e| e == "php").unwrap_or(false)
                    {
                        // Check if excluded
                        if !self.config.is_excluded(entry_path) {
                            files.push(entry_path.to_path_buf());
                        }
                    }
                }
            }
        }

        // Analyze files in parallel
        let results: Vec<_> = files
            .par_iter()
            .map(|file| self.analyze_file(file))
            .collect();

        // Combine results
        let mut combined = IssueCollection::new();
        for result in results {
            match result {
                Ok(issues) => combined.extend(issues.into_issues()),
                Err(e) => {
                    // Log error but continue
                    eprintln!("Warning: {}", e);
                }
            }
        }

        combined.sort();
        Ok(combined)
    }

    /// Analyze paths specified in the configuration
    pub fn analyze_configured_paths(&self) -> Result<IssueCollection, AnalyzeError> {
        let paths: Vec<_> = self.config.paths.iter().map(|p| p.as_path()).collect();
        if paths.is_empty() {
            return Err(AnalyzeError::NoPathsConfigured);
        }
        self.analyze_paths(&paths)
    }
}

/// Errors that can occur during analysis
#[derive(Debug, thiserror::Error)]
pub enum AnalyzeError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("No paths configured for analysis")]
    NoPathsConfigured,

    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::phpstan::ConfigError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = Analyzer::with_defaults();
        assert_eq!(analyzer.config.level, config::Level::Level0);
    }

    #[test]
    fn test_analyze_simple_php() {
        let analyzer = Analyzer::with_defaults();
        let source = "<?php\necho 'hello';\n";
        let issues = analyzer.analyze_source(Path::new("test.php"), source).unwrap();
        // No errors expected for valid PHP
        assert!(issues.is_empty() || issues.error_count() == 0);
    }

    #[test]
    fn test_analyze_undefined_function() {
        let analyzer = Analyzer::with_defaults();
        let source = "<?php\nmy_undefined_function();\n";
        let issues = analyzer.analyze_source(Path::new("test.php"), source).unwrap();
        // Should find undefined function
        assert!(issues.issues().iter().any(|i| i.message.contains("undefined function")));
    }
}
