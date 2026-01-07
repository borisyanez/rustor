//! rustor CLI - PHP refactoring tool
//!
//! Available rules:
//! - array_push: Convert array_push($arr, $val) to $arr[] = $val
//! - array_syntax: Convert array() to [] (short array syntax)
//! - empty_coalesce: Convert empty($x) ? $default : $x to $x ?: $default
//! - is_null: Convert is_null($x) to $x === null
//! - isset_coalesce: Convert isset($x) ? $x : $default to $x ?? $default
//! - join_to_implode: Convert join() to implode()
//! - pow_to_operator: Convert pow($x, $n) to $x ** $n
//! - sizeof: Convert sizeof($x) to count($x)
//! - type_cast: Convert strval/intval/floatval/boolval to cast syntax

mod config;
mod output;
mod process;

use anyhow::Result;
use clap::Parser;
use colored::*;
use std::path::PathBuf;
use std::process::ExitCode;

use config::Config;
use output::{OutputFormat, Reporter};
use process::{process_file, write_file};

/// All available rule names
const ALL_RULES: &[&str] = &[
    "array_push",
    "array_syntax",
    "empty_coalesce",
    "is_null",
    "isset_coalesce",
    "join_to_implode",
    "pow_to_operator",
    "sizeof",
    "type_cast",
];

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

    // Handle --list-rules
    if cli.list_rules {
        println!("{}", "Available rules:".bold());
        println!(
            "  {} - Convert array_push($arr, $val) to $arr[] = $val",
            "array_push".green()
        );
        println!(
            "  {} - Convert array() to [] (short array syntax)",
            "array_syntax".green()
        );
        println!(
            "  {} - Convert empty($x) ? $default : $x to $x ?: $default",
            "empty_coalesce".green()
        );
        println!(
            "  {} - Convert is_null($x) to $x === null",
            "is_null".green()
        );
        println!(
            "  {} - Convert isset($x) ? $x : $default to $x ?? $default",
            "isset_coalesce".green()
        );
        println!(
            "  {} - Convert join() to implode()",
            "join_to_implode".green()
        );
        println!(
            "  {} - Convert pow($x, $n) to $x ** $n",
            "pow_to_operator".green()
        );
        println!(
            "  {} - Convert sizeof($x) to count($x)",
            "sizeof".green()
        );
        println!(
            "  {} - Convert strval/intval/floatval/boolval to cast syntax",
            "type_cast".green()
        );
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

    // Determine which rules to run
    let enabled_rules = config.effective_rules(ALL_RULES, &cli.rule);

    // Validate rule names from CLI
    for rule in &cli.rule {
        if !ALL_RULES.contains(&rule.as_str()) {
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

    // Create reporter
    let mut reporter = Reporter::new(output_format, cli.verbose);

    // Process files
    for path in &cli.paths {
        if path.is_file() {
            process_single_file(path, &enabled_rules, &config, fix_mode, &mut reporter)?;
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "php"))
            {
                let file_path = entry.path();

                // Check exclude patterns
                if config.should_exclude(file_path) {
                    continue;
                }

                process_single_file(file_path, &enabled_rules, &config, fix_mode, &mut reporter)?;
            }
        } else if output_format == OutputFormat::Text {
            eprintln!(
                "{}: Path does not exist: {}",
                "Warning".yellow(),
                path.display()
            );
        }
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

fn process_single_file(
    path: &std::path::Path,
    enabled_rules: &std::collections::HashSet<String>,
    _config: &Config,
    fix_mode: bool,
    reporter: &mut Reporter,
) -> Result<()> {
    match process_file(path, enabled_rules) {
        Ok(Some(result)) => {
            if result.edits.is_empty() {
                reporter.report_skipped(path);
            } else if fix_mode {
                // Apply fixes
                if let Some(new_source) = &result.new_source {
                    write_file(path, new_source)?;
                }
                reporter.report_fix(path, result.edits);
            } else {
                // Check mode - show what would change
                reporter.report_check(
                    path,
                    result.edits,
                    &result.old_source,
                    result.new_source.as_deref().unwrap_or(&result.old_source),
                );
            }
        }
        Ok(None) => {
            // Parse error
            reporter.report_error(path, "Parse error, skipping");
        }
        Err(e) => {
            reporter.report_error(path, &format!("{:#}", e));
        }
    }
    Ok(())
}
