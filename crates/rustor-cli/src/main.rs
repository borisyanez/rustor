//! rustor CLI - PHP refactoring tool
//!
//! Available rules:
//! - array_push: Convert array_push($arr, $val) to $arr[] = $val
//! - array_syntax: Convert array() to [] (short array syntax)
//! - empty_coalesce: Convert empty($x) ? $default : $x to $x ?: $default
//! - is_null: Convert is_null($x) to $x === null
//! - isset_coalesce: Convert isset($x) ? $x : $default to $x ?? $default
//! - join_to_implode: Convert join() to implode()
//! - list_short_syntax: Convert list($a, $b) to [$a, $b]
//! - pow_to_operator: Convert pow($x, $n) to $x ** $n
//! - sizeof: Convert sizeof($x) to count($x)
//! - type_cast: Convert strval/intval/floatval/boolval to cast syntax

mod config;
mod output;
mod process;

use anyhow::Result;
use clap::Parser;
use colored::*;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use config::Config;
use output::{EditInfo, OutputFormat, Reporter};
use process::{process_file, write_file};
use rustor_rules::{Category, PhpVersion, Preset, RuleRegistry};

#[derive(Parser)]
#[command(name = "rustor")]
#[command(version = "0.2.0")]
#[command(about = "A Rust-based PHP refactoring tool")]
#[command(author = "rustor contributors")]
struct Cli {
    /// Files or directories to process
    #[arg(required_unless_present = "list_rules")]
    paths: Vec<PathBuf>,

    /// Check for issues without applying fixes (default mode)
    #[arg(long, conflicts_with = "fix")]
    check: bool,

    /// Apply fixes to files
    #[arg(long, conflicts_with = "check")]
    fix: bool,

    /// Show changes without applying them (alias for --check)
    #[arg(long, short = 'n', hide = true, conflicts_with = "fix")]
    dry_run: bool,

    /// Show verbose output
    #[arg(long, short = 'v')]
    verbose: bool,

    /// Rules to run (can be specified multiple times). Overrides config file.
    #[arg(long, short = 'r', value_name = "RULE")]
    rule: Vec<String>,

    /// Output format: text, json
    #[arg(long, value_name = "FORMAT", default_value = "text")]
    format: String,

    /// Shorthand for --format json
    #[arg(long, conflicts_with = "format")]
    json: bool,

    /// Path to config file (default: auto-detect .rustor.toml)
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Ignore config files
    #[arg(long)]
    no_config: bool,

    /// List available rules and exit
    #[arg(long)]
    list_rules: bool,

    /// Target PHP version (e.g., "7.4", "8.0"). Only rules compatible with this version will run.
    #[arg(long, value_name = "VERSION")]
    php_version: Option<String>,

    /// Only run rules in this category (performance, modernization, simplification, compatibility)
    #[arg(long, value_name = "CATEGORY")]
    category: Option<String>,

    /// Use a preset rule configuration (recommended, performance, modernize, all)
    #[arg(long, value_name = "PRESET")]
    preset: Option<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{}: {:#}", "Error".red(), e);
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    // Create rule registry
    let registry = RuleRegistry::new();

    // Handle --list-rules
    if cli.list_rules {
        println!("{}", "Available rules:".bold());
        for info in registry.list_rules_full() {
            let version_str = info
                .min_php_version
                .map(|v| format!(" [PHP {}+]", v))
                .unwrap_or_default();
            println!(
                "  {} - {} {}{}",
                info.name.green(),
                info.description,
                format!("[{}]", info.category).dimmed(),
                version_str.yellow()
            );
        }
        return Ok(ExitCode::SUCCESS);
    }

    // Determine output format
    let output_format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::from_str(&cli.format).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid output format '{}'. Valid options: text, json",
                cli.format
            )
        })?
    };

    // Load config file
    let config = if cli.no_config {
        Config::default()
    } else if let Some(config_path) = &cli.config {
        let cfg = Config::load_path(config_path)?;
        if cli.verbose && output_format == OutputFormat::Text {
            println!("{}: {}", "Using config".bold(), config_path.display());
        }
        cfg
    } else {
        match Config::load()? {
            Some((cfg, path)) => {
                if cli.verbose && output_format == OutputFormat::Text {
                    println!("{}: {}", "Using config".bold(), path.display());
                }
                cfg
            }
            None => Config::default(),
        }
    };

    // Get all available rule names from registry
    let all_rules = registry.all_names();

    // Parse preset (CLI overrides config)
    let preset: Option<Preset> = cli
        .preset
        .as_ref()
        .or(config.rules.preset.as_ref())
        .map(|p| {
            p.parse().unwrap_or_else(|e| {
                eprintln!("{}: {}", "Error".red(), e);
                std::process::exit(1);
            })
        });

    // Determine which rules to run
    // Priority: CLI --rule > preset > config enabled > all rules
    let enabled_rules: HashSet<String> = if !cli.rule.is_empty() {
        cli.rule.iter().cloned().collect()
    } else if let Some(preset) = preset {
        let preset_rules = registry.get_preset_rules(preset);
        // Apply disabled from config
        preset_rules
            .into_iter()
            .filter(|r| !config.rules.disabled.contains(r))
            .collect()
    } else {
        config.effective_rules(&all_rules, &cli.rule)
    };

    // Parse PHP version (CLI overrides config)
    let php_version: Option<PhpVersion> = cli
        .php_version
        .as_ref()
        .or(config.php.version.as_ref())
        .map(|v| {
            v.parse().unwrap_or_else(|e| {
                eprintln!("{}: {}", "Error".red(), e);
                std::process::exit(1);
            })
        });

    // Filter rules by PHP version
    let enabled_rules: HashSet<String> = if let Some(target_version) = php_version {
        enabled_rules
            .into_iter()
            .filter(|rule_name| {
                registry
                    .list_rules_full()
                    .iter()
                    .find(|r| r.name == rule_name)
                    .map(|r| {
                        r.min_php_version
                            .map(|v| v <= target_version)
                            .unwrap_or(true)
                    })
                    .unwrap_or(true)
            })
            .collect()
    } else {
        enabled_rules
    };

    // Parse category filter
    let category_filter: Option<Category> = cli.category.as_ref().map(|c| {
        c.parse().unwrap_or_else(|e| {
            eprintln!("{}: {}", "Error".red(), e);
            std::process::exit(1);
        })
    });

    // Filter rules by category
    let enabled_rules: HashSet<String> = if let Some(cat) = category_filter {
        enabled_rules
            .into_iter()
            .filter(|rule_name| {
                registry
                    .list_rules_full()
                    .iter()
                    .find(|r| r.name == rule_name)
                    .map(|r| r.category == cat)
                    .unwrap_or(false)
            })
            .collect()
    } else {
        enabled_rules
    };

    // Validate rule names from CLI
    for rule in &cli.rule {
        if !all_rules.contains(&rule.as_str()) {
            eprintln!(
                "{}: Unknown rule '{}'. Use --list-rules to see available rules.",
                "Error".red(),
                rule
            );
            return Ok(ExitCode::from(1));
        }
    }

    if enabled_rules.is_empty() {
        eprintln!("{}: No rules enabled", "Error".red());
        return Ok(ExitCode::from(1));
    }

    // Determine mode: fix or check (check is default)
    let fix_mode = cli.fix;
    let check_mode = !fix_mode; // --check, --dry-run, or default

    if cli.verbose && output_format == OutputFormat::Text {
        println!(
            "{}: {}",
            "Mode".bold(),
            if fix_mode { "fix" } else { "check" }
        );
        if let Some(v) = php_version {
            println!("{}: {}", "PHP Version".bold(), v);
        }
        println!(
            "{}: {}",
            "Rules".bold(),
            enabled_rules
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!();
    }

    // Collect all file paths first
    let mut file_paths: Vec<PathBuf> = Vec::new();
    let mut missing_paths: Vec<PathBuf> = Vec::new();

    for path in &cli.paths {
        if path.is_file() {
            file_paths.push(path.clone());
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "php"))
            {
                let file_path = entry.path();
                if !config.should_exclude(file_path) {
                    file_paths.push(file_path.to_path_buf());
                }
            }
        } else {
            missing_paths.push(path.clone());
        }
    }

    // Process files in parallel
    let results: Vec<FileResult> = file_paths
        .par_iter()
        .map(|path| process_file_to_result(path, &enabled_rules))
        .collect();

    // Sort results by path for deterministic output
    let mut sorted_results: Vec<_> = results.into_iter().zip(file_paths.iter()).collect();
    sorted_results.sort_by(|a, b| a.1.cmp(b.1));

    // Create reporter and process results sequentially
    let mut reporter = Reporter::new(output_format, cli.verbose);

    // Report missing paths
    for path in &missing_paths {
        if output_format == OutputFormat::Text {
            eprintln!(
                "{}: Path does not exist: {}",
                "Warning".yellow(),
                path.display()
            );
        }
    }

    // Report file results
    for (result, path) in sorted_results {
        report_result(path, result, fix_mode, &mut reporter)?;
    }

    // Determine exit code
    let summary = reporter.summary();
    let exit_code = if summary.errors > 0 {
        ExitCode::from(1)
    } else if check_mode && summary.files_with_changes > 0 {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    };

    // Print final output
    reporter.finish(check_mode);

    Ok(exit_code)
}

/// Result of processing a single file (for parallel processing)
enum FileResult {
    /// File had no changes
    NoChanges,
    /// File has changes to report/apply
    HasChanges {
        edits: Vec<EditInfo>,
        old_source: String,
        new_source: String,
    },
    /// Parse error occurred
    ParseError,
    /// Other error occurred
    Error(String),
}

/// Process a file and return a result (no I/O, suitable for parallel execution)
fn process_file_to_result(path: &PathBuf, enabled_rules: &HashSet<String>) -> FileResult {
    match process_file(path, enabled_rules) {
        Ok(Some(result)) => {
            if result.edits.is_empty() {
                FileResult::NoChanges
            } else {
                FileResult::HasChanges {
                    edits: result.edits,
                    old_source: result.old_source,
                    new_source: result.new_source.unwrap_or_default(),
                }
            }
        }
        Ok(None) => FileResult::ParseError,
        Err(e) => FileResult::Error(format!("{:#}", e)),
    }
}

/// Report a file result and optionally apply fixes
fn report_result(
    path: &PathBuf,
    result: FileResult,
    fix_mode: bool,
    reporter: &mut Reporter,
) -> Result<()> {
    match result {
        FileResult::NoChanges => {
            reporter.report_skipped(path);
        }
        FileResult::HasChanges {
            edits,
            old_source,
            new_source,
        } => {
            if fix_mode {
                write_file(path, &new_source)?;
                reporter.report_fix(path, edits);
            } else {
                reporter.report_check(path, edits, &old_source, &new_source);
            }
        }
        FileResult::ParseError => {
            reporter.report_error(path, "Parse error, skipping");
        }
        FileResult::Error(msg) => {
            reporter.report_error(path, &msg);
        }
    }
    Ok(())
}
