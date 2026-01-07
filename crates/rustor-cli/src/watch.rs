//! Watch mode for rustor - re-run analysis on file changes
//!
//! Uses the `notify` crate for cross-platform file watching with debouncing.

use anyhow::Result;
use colored::*;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

use crate::cache::{hash_file, hash_rules, Cache};
use crate::output::OutputFormat;
use crate::process::process_file;

/// Configuration for watch mode
pub struct WatchConfig {
    /// Paths to watch
    pub paths: Vec<PathBuf>,
    /// Enabled rules
    pub enabled_rules: HashSet<String>,
    /// Output format
    pub format: OutputFormat,
    /// Verbose output
    #[allow(dead_code)]
    pub verbose: bool,
    /// Debounce duration (default 100ms)
    pub debounce: Duration,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            paths: vec![],
            enabled_rules: HashSet::new(),
            format: OutputFormat::Text,
            verbose: false,
            debounce: Duration::from_millis(100),
        }
    }
}

/// Run watch mode - monitors files and re-runs analysis on changes
pub fn run_watch(config: WatchConfig) -> Result<()> {
    let (tx, rx) = channel();

    // Create a debounced file watcher
    let mut debouncer = new_debouncer(config.debounce, tx)?;

    // Watch all specified paths
    for path in &config.paths {
        if path.is_dir() {
            debouncer.watcher().watch(path, RecursiveMode::Recursive)?;
            if config.format == OutputFormat::Text {
                println!("{} Watching: {}", "→".cyan(), path.display());
            }
        } else if path.is_file() {
            if let Some(parent) = path.parent() {
                debouncer.watcher().watch(parent, RecursiveMode::NonRecursive)?;
            }
            if config.format == OutputFormat::Text {
                println!("{} Watching: {}", "→".cyan(), path.display());
            }
        }
    }

    let rules_hash = hash_rules(&config.enabled_rules);
    let mut cache = Cache::default();

    if config.format == OutputFormat::Text {
        println!();
        println!("{}", "Watching for changes (Ctrl+C to stop)...".dimmed());
        println!();
    }

    // Initial run
    run_analysis(&config, &mut cache, rules_hash)?;

    // Watch loop
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                let mut changed_files: Vec<PathBuf> = Vec::new();

                for event in events {
                    if event.kind == DebouncedEventKind::Any {
                        let path = &event.path;
                        if path.extension().is_some_and(|ext| ext == "php") {
                            if !changed_files.contains(path) {
                                changed_files.push(path.clone());
                            }
                        }
                    }
                }

                if !changed_files.is_empty() {
                    // Clear screen for fresh output
                    if config.format == OutputFormat::Text {
                        print!("\x1B[2J\x1B[1;1H"); // Clear screen and move cursor to top
                        println!("{}", "File changes detected, re-analyzing...".cyan());
                        println!();
                    }

                    // Invalidate cache for changed files
                    for path in &changed_files {
                        cache.entries.remove(path);
                    }

                    // Re-run analysis
                    run_analysis(&config, &mut cache, rules_hash)?;

                    if config.format == OutputFormat::Text {
                        println!();
                        println!("{}", "Watching for changes (Ctrl+C to stop)...".dimmed());
                    }
                }
            }
            Ok(Err(error)) => {
                eprintln!("{}: Watch error: {:?}", "Error".red(), error);
            }
            Err(_) => {
                // Channel closed, exit cleanly
                break;
            }
        }
    }

    Ok(())
}

/// Run analysis on all watched files
fn run_analysis(config: &WatchConfig, cache: &mut Cache, rules_hash: u64) -> Result<()> {
    let mut files_with_changes = 0;
    let mut total_edits = 0;

    // Collect all PHP files
    let mut file_paths: Vec<PathBuf> = Vec::new();
    for path in &config.paths {
        if path.is_file() {
            file_paths.push(path.clone());
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "php"))
            {
                file_paths.push(entry.path().to_path_buf());
            }
        }
    }

    // Process files
    for path in &file_paths {
        // Check cache first
        if let Ok(content_hash) = hash_file(path) {
            if let Some(entry) = cache.get_if_valid(path, content_hash, rules_hash) {
                if entry.has_edits {
                    files_with_changes += 1;
                    total_edits += entry.edit_count;
                    if config.format == OutputFormat::Text {
                        println!("{}", path.display().to_string().bold());
                        println!(
                            "  {} {} change(s) (cached)",
                            "!".yellow(),
                            entry.edit_count
                        );
                        println!();
                    }
                }
                continue;
            }
        }

        // Process file
        match process_file(path, &config.enabled_rules) {
            Ok(Some(result)) => {
                if !result.edits.is_empty() {
                    files_with_changes += 1;
                    total_edits += result.edits.len();

                    if config.format == OutputFormat::Text {
                        println!("{}", path.display().to_string().bold());
                        for edit in &result.edits {
                            println!("  {} {}", "->".green(), edit.message);
                        }
                        println!();
                    }

                    // Update cache
                    if let Ok(content_hash) = hash_file(path) {
                        cache.update(path.clone(), content_hash, rules_hash, true, result.edits.len());
                    }
                } else {
                    // No changes - update cache
                    if let Ok(content_hash) = hash_file(path) {
                        cache.update(path.clone(), content_hash, rules_hash, false, 0);
                    }
                }
            }
            Ok(None) => {
                // Parse error - skip silently in watch mode
            }
            Err(_) => {
                // Error - skip silently in watch mode
            }
        }
    }

    // Print summary
    if config.format == OutputFormat::Text {
        if files_with_changes > 0 {
            println!(
                "{}: {} file(s) with {} total change(s)",
                "Summary".bold(),
                files_with_changes,
                total_edits
            );
        } else {
            println!("{}: No changes found", "Summary".bold());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_config_default() {
        let config = WatchConfig::default();
        assert!(config.paths.is_empty());
        assert!(config.enabled_rules.is_empty());
        assert_eq!(config.format, OutputFormat::Text);
        assert!(!config.verbose);
        assert_eq!(config.debounce, Duration::from_millis(100));
    }
}
