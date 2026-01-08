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

mod analyze;
mod baseline;
mod cache;
mod backup;
mod config;
mod fixer;
mod git;
mod ignore;
mod lsp;
mod output;
mod plugin;
mod process;
mod watch;

use anyhow::Result;
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Mutex;

use cache::{hash_file, hash_rules, Cache};
use config::Config;
use output::{EditInfo, OutputFormat, Reporter};
use process::{process_file_with_skip, write_file};
use rustor_rules::{Category, PhpVersion, Preset, RuleConfigs, RuleRegistry};

#[derive(Parser)]
#[command(name = "rustor")]
#[command(version = "0.2.0")]
#[command(about = "A Rust-based PHP refactoring tool")]
#[command(author = "rustor contributors")]
struct Cli {
    /// Files or directories to process
    #[arg(required_unless_present_any = ["list_rules", "list_fixers", "staged", "since"])]
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

    /// Output format: text, json, diff, sarif, html, checkstyle, github
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

    /// Disable caching (always re-process all files)
    #[arg(long)]
    no_cache: bool,

    /// Clear the cache before running
    #[arg(long)]
    clear_cache: bool,

    /// Watch mode: re-run analysis when files change
    #[arg(long, short = 'w')]
    watch: bool,

    /// Only check git-staged files (for pre-commit hooks)
    #[arg(long)]
    staged: bool,

    /// Only check files changed since this git ref (branch, tag, or commit)
    #[arg(long, value_name = "REF")]
    since: Option<String>,

    /// Disable progress bar output
    #[arg(long)]
    no_progress: bool,

    /// Generate baseline file to stdout (captures current issues)
    #[arg(long)]
    generate_baseline: bool,

    /// Use baseline file to filter results (only show new issues)
    #[arg(long, value_name = "FILE")]
    baseline: Option<PathBuf>,

    /// Create backup files before applying fixes (default: true)
    #[arg(long, default_value = "true")]
    backup: bool,

    /// Disable backup creation when fixing files
    #[arg(long, conflicts_with = "backup")]
    no_backup: bool,

    /// Directory to store backup files (default: .rustor-backups)
    #[arg(long, value_name = "DIR")]
    backup_dir: Option<PathBuf>,

    /// Verify fixed files parse correctly (restore on failure)
    #[arg(long)]
    verify: bool,

    /// Run as Language Server Protocol (LSP) server for IDE integration
    #[arg(long)]
    lsp: bool,

    // Fixer options
    /// Path to .php-cs-fixer.php config file for formatting rules
    #[arg(long, value_name = "PATH")]
    fixer_config: Option<PathBuf>,

    /// Run formatting fixers (PSR-12 compatible)
    #[arg(long)]
    fixer: bool,

    /// Fixer preset to use (psr12, symfony, phpcsfixer)
    #[arg(long, value_name = "PRESET")]
    fixer_preset: Option<String>,

    /// List available formatters/fixers
    #[arg(long)]
    list_fixers: bool,
}

fn main() -> ExitCode {
    // Check for LSP mode early (before parsing other args)
    if std::env::args().any(|arg| arg == "--lsp") {
        tokio::runtime::Runtime::new()
            .expect("Failed to create tokio runtime")
            .block_on(lsp::run_lsp_server());
        return ExitCode::SUCCESS;
    }

    // Check for analyze subcommand early
    let args: Vec<String> = std::env::args().skip(1).collect();
    if analyze::should_run_analyze(&args) {
        let analyze_args = args.into_iter().skip(1).collect::<Vec<_>>();
        match analyze::parse_analyze_args(&analyze_args) {
            Ok(args) => match analyze::run_analyze(args) {
                Ok(code) => return code,
                Err(e) => {
                    eprintln!("{}: {:#}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            },
            Err(e) => {
                eprintln!("{}: {:#}", "Error".red(), e);
                return ExitCode::from(1);
            }
        }
    }

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

    // Determine output format early (needed for verbose output)
    let output_format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::from_str(&cli.format).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid output format '{}'. Valid options: text, json, diff, sarif, html, checkstyle, github",
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

    // Convert rule-specific config options
    let rule_configs: RuleConfigs = config.rules.to_rule_configs();

    // Create rule registry with configuration
    let registry = RuleRegistry::new_with_config(&rule_configs);

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

    // Handle --list-fixers
    if cli.list_fixers {
        let fixer_registry = rustor_fixer::FixerRegistry::new();
        fixer::list_fixers(&fixer_registry);
        return Ok(ExitCode::SUCCESS);
    }

    // Handle --fixer mode (run formatting fixers only)
    if cli.fixer {
        return run_fixer_mode(&cli, output_format);
    }

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

    // Handle watch mode
    if cli.watch {
        let watch_config = watch::WatchConfig {
            paths: cli.paths.clone(),
            enabled_rules: enabled_rules.clone(),
            format: output_format,
            verbose: cli.verbose,
            ..Default::default()
        };
        watch::run_watch(watch_config)?;
        return Ok(ExitCode::SUCCESS);
    }

    // Determine mode: fix or check (check is default)
    let fix_mode = cli.fix;
    let check_mode = !fix_mode; // --check, --dry-run, or default

    // Set up backup manager for fix mode
    let backup_enabled = fix_mode && cli.backup && !cli.no_backup;
    let backup_dir = cli.backup_dir
        .clone()
        .or_else(|| config.fix.backup_dir.clone().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(".rustor-backups"));
    let mut backup_manager = backup::BackupManager::new(backup_dir.clone(), backup_enabled);

    // Initialize backup session if we're in fix mode with backups
    if backup_enabled {
        backup_manager.init_session()?;
        if cli.verbose && output_format == OutputFormat::Text {
            if let Some(session_path) = backup_manager.session_path() {
                println!("{}: {}", "Backup dir".bold(), session_path.display());
            }
        }
    }

    // Determine cache directory (use cwd)
    let cache_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Handle --clear-cache
    if cli.clear_cache {
        if let Err(e) = cache::clear_cache(&cache_dir) {
            if cli.verbose && output_format == OutputFormat::Text {
                eprintln!("{}: Failed to clear cache: {}", "Warning".yellow(), e);
            }
        } else if cli.verbose && output_format == OutputFormat::Text {
            println!("{}: Cache cleared", "Info".bold());
        }
    }

    // Load cache (unless disabled)
    let use_cache = !cli.no_cache;
    let cache = if use_cache {
        Cache::load(&cache_dir).unwrap_or_default()
    } else {
        Cache::default()
    };
    let cache = Mutex::new(cache);

    // Compute rules hash for cache invalidation
    let rules_hash = hash_rules(&enabled_rules);

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

    // Git mode: --staged or --since
    if cli.staged || cli.since.is_some() {
        let repo_root = match git::find_repo_root() {
            Ok(root) => root,
            Err(e) => {
                eprintln!("{}: {}", "Error".red(), e);
                return Ok(ExitCode::from(1));
            }
        };

        let git_files = if cli.staged {
            if cli.verbose && output_format == OutputFormat::Text {
                println!("{}: Checking staged files", "Git".bold());
            }
            git::get_staged_files(&repo_root)
        } else {
            let ref_name = cli.since.as_ref().unwrap();
            if cli.verbose && output_format == OutputFormat::Text {
                println!("{}: Checking files changed since {}", "Git".bold(), ref_name);
            }
            git::get_changed_files_since(&repo_root, ref_name)
        };

        match git_files {
            Ok(files) => {
                for path in files {
                    if !config.should_exclude(&path) {
                        file_paths.push(path);
                    }
                }
            }
            Err(e) => {
                eprintln!("{}: {}", "Error".red(), e);
                return Ok(ExitCode::from(1));
            }
        }

        if file_paths.is_empty() {
            if output_format == OutputFormat::Text {
                println!("{}", "No PHP files found to check".dimmed());
            }
            return Ok(ExitCode::SUCCESS);
        }
    } else {
        // Normal mode: process paths from command line
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
    }

    // Create progress bar (only for text format with TTY and not disabled)
    let show_progress = !cli.no_progress
        && output_format == OutputFormat::Text
        && atty::is(atty::Stream::Stdout)
        && file_paths.len() > 10;  // Only show for larger scans

    let progress = if show_progress {
        let pb = ProgressBar::new(file_paths.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} Scanning... [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%)"
            )
            .unwrap()
            .progress_chars("█▓░")
        );
        Some(pb)
    } else {
        None
    };

    // Process files in parallel (with caching)
    let cache_hits = Mutex::new(0usize);
    let results: Vec<FileResult> = file_paths
        .par_iter()
        .map(|path| {
            // Check cache first
            if use_cache {
                if let Ok(content_hash) = hash_file(path) {
                    let cache_guard = cache.lock().unwrap();
                    if let Some(entry) = cache_guard.get_if_valid(path, content_hash, rules_hash) {
                        // Cache hit - use cached result
                        *cache_hits.lock().unwrap() += 1;
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        return if entry.has_edits {
                            FileResult::CachedWithEdits { edit_count: entry.edit_count }
                        } else {
                            FileResult::NoChanges
                        };
                    }
                }
            }

            // Cache miss - process the file
            let result = process_file_to_result(path, &enabled_rules, &rule_configs, &config);

            // Update cache with result
            if use_cache {
                if let Ok(content_hash) = hash_file(path) {
                    let (has_edits, edit_count) = match &result {
                        FileResult::HasChanges { edits, .. } => (true, edits.len()),
                        FileResult::NoChanges => (false, 0),
                        _ => (false, 0),
                    };
                    let mut cache_guard = cache.lock().unwrap();
                    cache_guard.update(path.clone(), content_hash, rules_hash, has_edits, edit_count);
                }
            }

            if let Some(ref pb) = progress {
                pb.inc(1);
            }

            result
        })
        .collect();

    // Clear progress bar before output
    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    // Report cache stats in verbose mode
    let hits = *cache_hits.lock().unwrap();
    if cli.verbose && use_cache && hits > 0 && output_format == OutputFormat::Text {
        println!(
            "{}: {} files skipped (unchanged)",
            "Cache".bold(),
            hits
        );
    }

    // Sort results by path for deterministic output
    let mut sorted_results: Vec<_> = results.into_iter().zip(file_paths.iter()).collect();
    sorted_results.sort_by(|a, b| a.1.cmp(b.1));

    // Load baseline if specified
    let loaded_baseline = if let Some(baseline_path) = &cli.baseline {
        Some(baseline::Baseline::load(baseline_path)?)
    } else {
        None
    };

    // Create reporter and process results sequentially
    // Use a silent reporter for baseline generation
    let reporter_format = if cli.generate_baseline {
        OutputFormat::Json  // Suppress text output during baseline generation
    } else {
        output_format
    };
    let mut reporter = Reporter::new(reporter_format, cli.verbose && !cli.generate_baseline);
    reporter.set_enabled_rules(enabled_rules.iter().cloned().collect());

    // Collect data for baseline generation if requested
    let mut baseline_data: Vec<(String, Vec<EditInfo>, Option<String>)> = Vec::new();

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
        // Apply baseline filtering if baseline is loaded
        let result = if let Some(ref baseline) = loaded_baseline {
            apply_baseline_filter(result, path, baseline)
        } else {
            result
        };

        // Collect for baseline generation
        if cli.generate_baseline {
            if let FileResult::HasChanges { ref edits, ref old_source, .. } = result {
                baseline_data.push((
                    path.display().to_string(),
                    edits.clone(),
                    Some(old_source.clone()),
                ));
            }
        }

        report_result(path, result, fix_mode, &mut reporter, &backup_manager, cli.verify)?;
    }

    // Save cache
    if use_cache {
        let cache = cache.into_inner().unwrap();
        if let Err(e) = cache.save(&cache_dir) {
            if cli.verbose && output_format == OutputFormat::Text {
                eprintln!("{}: Failed to save cache: {}", "Warning".yellow(), e);
            }
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
    if cli.generate_baseline {
        // Generate baseline instead of normal output
        let baseline = baseline::Baseline::generate(&baseline_data);
        println!("{}", baseline.to_json()?);
    } else {
        reporter.finish(check_mode);
    }

    Ok(exit_code)
}

/// Apply baseline filtering to a file result
fn apply_baseline_filter(result: FileResult, path: &PathBuf, baseline: &baseline::Baseline) -> FileResult {
    match result {
        FileResult::HasChanges { edits, old_source, new_source } => {
            let path_str = path.display().to_string();
            let filtered_edits = baseline.filter_edits(&path_str, edits, &old_source);

            if filtered_edits.is_empty() {
                FileResult::NoChanges
            } else {
                // Re-apply only filtered edits to get correct new_source
                // For simplicity, we still report all edits' new_source
                // since partial application is complex
                FileResult::HasChanges {
                    edits: filtered_edits,
                    old_source,
                    new_source,
                }
            }
        }
        other => other,
    }
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
    /// Cached result with edits (we don't have the details, just the count)
    CachedWithEdits { edit_count: usize },
    /// Parse error occurred
    ParseError,
    /// Other error occurred
    Error(String),
}

/// Process a file and return a result (no I/O, suitable for parallel execution)
fn process_file_to_result(
    path: &PathBuf,
    enabled_rules: &HashSet<String>,
    rule_configs: &RuleConfigs,
    config: &Config,
) -> FileResult {
    // Check if all rules should be skipped for this path
    if config.should_skip_all_rules(path) {
        return FileResult::NoChanges;
    }

    // Get rules to skip for this specific path
    let skip_rules = config.skipped_rules_for_path(path);

    match process_file_with_skip(path, enabled_rules, rule_configs, &skip_rules) {
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
    backup_manager: &backup::BackupManager,
    verify: bool,
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
                // Create backup before modifying
                let backup_path = backup_manager.backup_file(path)?;

                // Write the fixed file
                write_file(path, &new_source)?;

                // Verify if requested
                if verify {
                    if !backup::verify_php_file(path)? {
                        // Restore from backup on parse failure
                        if let Some(bp) = backup_path {
                            backup_manager.restore_file(path, &bp)?;
                            reporter.report_error(path, "Fix produced invalid PHP, restored from backup");
                            return Ok(());
                        } else {
                            reporter.report_error(path, "Fix produced invalid PHP (no backup to restore)");
                            return Ok(());
                        }
                    }
                }

                reporter.report_fix(path, edits);
            } else {
                reporter.report_check(path, edits, &old_source, &new_source);
            }
        }
        FileResult::CachedWithEdits { edit_count } => {
            // File was cached with edits - report as having changes but no details
            reporter.report_cached(path, edit_count);
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

/// Run fixer-only mode (formatting fixers, no refactoring rules)
fn run_fixer_mode(cli: &Cli, output_format: OutputFormat) -> Result<ExitCode> {
    use rustor_fixer::{FixerRegistry, FixerConfig};
    use rustor_fixer::config::LineEnding;
    use rayon::prelude::*;

    let fixer_registry = FixerRegistry::new();

    // Get fixer preset or use all fixers
    let fixer_preset = cli.fixer_preset.as_deref().unwrap_or("psr12");

    // Create fixer config
    let fixer_config = FixerConfig {
        line_ending: LineEnding::Lf,
        ..Default::default()
    };

    // Collect PHP files
    let mut files: Vec<PathBuf> = Vec::new();
    for path in &cli.paths {
        if path.is_file() {
            files.push(path.clone());
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "php"))
            {
                files.push(entry.path().to_path_buf());
            }
        }
    }

    if files.is_empty() {
        if output_format == OutputFormat::Text {
            println!("{}", "No PHP files found".yellow());
        }
        return Ok(ExitCode::SUCCESS);
    }

    let file_count = files.len();

    // Process files in parallel
    let results: Vec<(PathBuf, Vec<rustor_core::Edit>)> = files
        .par_iter()
        .filter_map(|path| {
            let source = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => return None,
            };

            let edits = fixer_registry.check_preset(&source, fixer_preset, &fixer_config);

            if edits.is_empty() {
                None
            } else {
                Some((path.clone(), edits))
            }
        })
        .collect();

    // Output results
    let mut total_edits = 0;
    let mut files_with_changes = 0;

    match output_format {
        OutputFormat::Json => {
            use serde_json::json;

            let file_results: Vec<_> = results
                .iter()
                .map(|(path, edits)| {
                    total_edits += edits.len();
                    files_with_changes += 1;
                    json!({
                        "path": path.display().to_string(),
                        "edits": edits.iter().map(|e| {
                            json!({
                                "rule": e.rule.as_deref().unwrap_or("unknown"),
                                "message": e.message,
                            })
                        }).collect::<Vec<_>>()
                    })
                })
                .collect();

            let output = json!({
                "version": env!("CARGO_PKG_VERSION"),
                "summary": {
                    "files_processed": file_count,
                    "files_with_changes": files_with_changes,
                    "total_edits": total_edits,
                },
                "files": file_results
            });

            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Text => {
            for (path, edits) in &results {
                files_with_changes += 1;
                total_edits += edits.len();
                println!(
                    "{}",
                    format!("{}", path.display()).cyan()
                );
                for edit in edits {
                    println!(
                        "  {} {}",
                        format!("[{}]", edit.rule.as_deref().unwrap_or("fixer")).yellow(),
                        edit.message
                    );
                }
            }

            println!();
            println!(
                "{}: {} file(s), {} edit(s)",
                "Summary".bold(),
                files_with_changes,
                total_edits
            );
        }
        OutputFormat::Diff => {
            for (path, edits) in &results {
                let source = std::fs::read_to_string(path).unwrap_or_default();
                if let Ok(fixed) = rustor_core::apply_edits(&source, edits) {
                    println!("--- a/{}", path.display());
                    println!("+++ b/{}", path.display());
                    // Simple diff output
                    for (i, (old, new)) in source.lines().zip(fixed.lines()).enumerate() {
                        if old != new {
                            println!("@@ -{},{} +{},{} @@", i + 1, 1, i + 1, 1);
                            println!("-{}", old);
                            println!("+{}", new);
                        }
                    }
                }
            }
        }
        _ => {
            // For other formats, just output basic info
            for (path, edits) in &results {
                println!("{}: {} edit(s)", path.display(), edits.len());
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
