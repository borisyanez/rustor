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

pub mod autoload;
pub mod baseline;
pub mod checks;
pub mod config;
pub mod issue;
pub mod logging;
pub mod output;
pub mod resolver;
pub mod scope;
pub mod symbols;
pub mod types;

use autoload::AutoloadScanner;
use checks::{CheckContext, CheckRegistry, PHP_BUILTIN_CLASSES, PHP_BUILTIN_FUNCTIONS};
use config::composer::ComposerJson;
use config::PhpStanConfig;
use issue::IssueCollection;
use mago_database::file::FileId;
use rayon::prelude::*;
use resolver::symbol_collector::SymbolCollector;
use symbols::SymbolTable;
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
            symbol_table: None, // TODO: Pass symbol table for cross-file analysis
            scope: None,        // TODO: Pass scope for variable tracking
            analysis_level: self.config.level.as_u8(),
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

        // Load autoload symbols from composer.json if available
        let mut symbol_table = self.load_autoload_symbols(paths);

        // First pass: collect symbols from all files to build symbol table
        let collected_symbols: Vec<_> = files
            .par_iter()
            .filter_map(|file| {
                let source = fs::read_to_string(file).ok()?;
                let arena = bumpalo::Bump::new();
                let file_id = FileId::new(file.to_string_lossy().as_ref());
                let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, &source);

                let collector = SymbolCollector::new(&source, file);
                Some(collector.collect(&program))
            })
            .collect();

        // Merge target file symbols into autoload symbol table
        let target_symbols = SymbolCollector::build_symbol_table_from_symbols(collected_symbols);
        symbol_table.merge(target_symbols);

        // Collect symbols from files included via require/include statements
        let include_symbols = self.collect_include_symbols(&files);
        symbol_table.merge(include_symbols);

        // Second pass: analyze files with symbol table
        let results: Vec<_> = files
            .par_iter()
            .map(|file| self.analyze_file_with_symbols(file, &symbol_table))
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

    /// Analyze a file with a pre-built symbol table
    fn analyze_file_with_symbols(&self, path: &Path, symbol_table: &SymbolTable) -> Result<IssueCollection, AnalyzeError> {
        let source = fs::read_to_string(path)?;
        self.analyze_source_with_symbols(path, &source, symbol_table)
    }

    /// Analyze source code with a given path and symbol table
    fn analyze_source_with_symbols(&self, path: &Path, source: &str, symbol_table: &SymbolTable) -> Result<IssueCollection, AnalyzeError> {
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

        // Create check context with symbol table
        let ctx = CheckContext {
            file_path: path,
            source,
            config: &self.config,
            builtin_functions: PHP_BUILTIN_FUNCTIONS,
            builtin_classes: PHP_BUILTIN_CLASSES,
            symbol_table: Some(symbol_table),
            scope: None,        // TODO: Pass scope for variable tracking
            analysis_level: self.config.level.as_u8(),
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

    /// Analyze paths specified in the configuration
    pub fn analyze_configured_paths(&self) -> Result<IssueCollection, AnalyzeError> {
        let paths: Vec<_> = self.config.paths.iter().map(|p| p.as_path()).collect();
        if paths.is_empty() {
            return Err(AnalyzeError::NoPathsConfigured);
        }
        self.analyze_paths(&paths)
    }

    /// Collect symbols from files included via require/include statements
    fn collect_include_symbols(&self, files: &[std::path::PathBuf]) -> SymbolTable {
        use autoload::include_scanner::IncludeScanner;
        use std::collections::HashSet;

        // First pass: parallel scan to find all includes from target files
        let initial_includes: Vec<Vec<std::path::PathBuf>> = files
            .par_iter()
            .filter_map(|file| {
                let source = fs::read_to_string(file).ok()?;
                let arena = bumpalo::Bump::new();
                let file_id = FileId::new(file.to_string_lossy().as_ref());
                let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, &source);

                let scanner = IncludeScanner::new(&source, file);
                let includes = scanner.scan(&program);
                if includes.is_empty() {
                    None
                } else {
                    Some(includes)
                }
            })
            .collect();

        // Flatten and dedupe
        let mut all_includes: HashSet<std::path::PathBuf> = HashSet::new();
        let mut to_process: Vec<std::path::PathBuf> = Vec::new();
        for includes in initial_includes {
            for include_path in includes {
                if !all_includes.contains(&include_path) {
                    all_includes.insert(include_path.clone());
                    to_process.push(include_path);
                }
            }
        }

        // Recursively process includes (up to 3 levels deep to avoid infinite loops)
        // Use parallel processing for each batch
        for _ in 0..3 {
            let current_batch: Vec<_> = to_process.drain(..).collect();
            if current_batch.is_empty() {
                break;
            }

            let new_includes: Vec<Vec<std::path::PathBuf>> = current_batch
                .par_iter()
                .filter_map(|file| {
                    let source = fs::read_to_string(file).ok()?;
                    let arena = bumpalo::Bump::new();
                    let file_id = FileId::new(file.to_string_lossy().as_ref());
                    let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, &source);

                    let scanner = IncludeScanner::new(&source, file);
                    let includes = scanner.scan(&program);
                    if includes.is_empty() {
                        None
                    } else {
                        Some(includes)
                    }
                })
                .collect();

            // Merge results
            for includes in new_includes {
                for include_path in includes {
                    if !all_includes.contains(&include_path) {
                        all_includes.insert(include_path.clone());
                        to_process.push(include_path);
                    }
                }
            }
        }

        // Now collect symbols from all discovered include files (already parallel)
        let collected: Vec<_> = all_includes
            .par_iter()
            .filter_map(|file| {
                let source = fs::read_to_string(file).ok()?;
                let arena = bumpalo::Bump::new();
                let file_id = FileId::new(file.to_string_lossy().as_ref());
                let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, &source);

                let collector = SymbolCollector::new(&source, file);
                Some(collector.collect(&program))
            })
            .collect();

        SymbolCollector::build_symbol_table_from_symbols(collected)
    }

    /// Load symbols from Composer autoload (classmap + PSR-4 + vendor PSR-4)
    fn load_autoload_symbols(&self, paths: &[&Path]) -> SymbolTable {
        let search_dir = paths.first()
            .map(|p| if p.is_file() { p.parent().unwrap_or(*p) } else { *p })
            .unwrap_or(Path::new("."));

        let mut symbol_table = SymbolTable::new();

        // Try to find vendor and classmap paths
        let classmap_scanner = autoload::ClassmapScanner::find_from_directory(search_dir);
        let vendor_psr4_scanner = autoload::VendorPsr4Scanner::find_from_directory(search_dir);

        // Check if we can use cached vendor symbols
        if let (Some(ref cm_scanner), Some(ref vp_scanner)) = (&classmap_scanner, &vendor_psr4_scanner) {
            let cache = autoload::cache::AutoloadCache::for_project(search_dir);
            if let Some(cached) = cache.load(&vp_scanner.vendor_dir(), &cm_scanner.classmap_path()) {
                eprintln!("Autoload: using cached vendor symbols ({} classes)", cached.all_classes().count());
                symbol_table.merge(cached);

                // Still load project PSR-4 (fast)
                let composer_path = ComposerJson::find_in_directory(search_dir);
                if let Some(path) = composer_path {
                    if let Ok(composer) = ComposerJson::load(&path) {
                        if composer.has_autoload() {
                            let base_dir = path.parent().unwrap_or(Path::new("."));
                            let scanner = AutoloadScanner::from_composer(&composer, base_dir, true);
                            let stats = scanner.stats();
                            if stats.file_count > 0 {
                                eprintln!(
                                    "Autoload: scanning {} files from {} PSR-4 mappings",
                                    stats.file_count, stats.mapping_count
                                );
                                symbol_table.merge(scanner.build_symbol_table());
                            }
                        }
                    }
                }

                return symbol_table;
            }
        }

        // No cache - build from scratch
        // First, load from Composer's classmap (vendor classes)
        if let Some(ref classmap_scanner) = classmap_scanner {
            let stats = classmap_scanner.stats();
            eprintln!(
                "Autoload: loading {} classes from {}",
                stats.class_count,
                stats.classmap_path.display()
            );
            symbol_table.merge(classmap_scanner.build_symbol_table());
        }

        // Load vendor PSR-4 symbols (interfaces, abstract classes not in classmap)
        if let Some(ref vp_scanner) = vendor_psr4_scanner {
            let stats = vp_scanner.stats();
            eprintln!(
                "Autoload: scanning {} vendor PSR-4 mappings (this may take a moment...)",
                stats.mapping_count
            );
            let vendor_table = vp_scanner.build_symbol_table();
            eprintln!(
                "Autoload: found {} classes in vendor PSR-4 directories",
                vendor_table.all_classes().count()
            );
            symbol_table.merge(vendor_table);
        }

        // Save to cache for next time
        if let (Some(ref cm_scanner), Some(ref vp_scanner)) = (&classmap_scanner, &vendor_psr4_scanner) {
            let cache = autoload::cache::AutoloadCache::for_project(search_dir);
            if let Err(e) = cache.save(&symbol_table, &vp_scanner.vendor_dir(), &cm_scanner.classmap_path()) {
                eprintln!("Autoload: failed to save cache: {}", e);
            } else {
                eprintln!("Autoload: saved symbol cache for faster future runs");
            }
        }

        // Then, project PSR-4 autoload from composer.json
        let composer_path = ComposerJson::find_in_directory(search_dir);
        if let Some(path) = composer_path {
            if let Ok(composer) = ComposerJson::load(&path) {
                if composer.has_autoload() {
                    let base_dir = path.parent().unwrap_or(Path::new("."));
                    let scanner = AutoloadScanner::from_composer(&composer, base_dir, true);
                    let stats = scanner.stats();
                    if stats.file_count > 0 {
                        eprintln!(
                            "Autoload: scanning {} files from {} PSR-4 mappings",
                            stats.file_count, stats.mapping_count
                        );
                        symbol_table.merge(scanner.build_symbol_table());
                    }
                }
            }
        }

        symbol_table
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
        // Should find undefined function (message: "Function X not found.")
        assert!(issues.issues().iter().any(|i| i.message.contains("not found")));
    }
}
