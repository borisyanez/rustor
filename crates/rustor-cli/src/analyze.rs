//! Static analysis subcommand (PHPStan-compatible)

use anyhow::Result;
use colored::*;
use rustor_analyze::{
    baseline::Baseline,
    config::{Level, PhpStanConfig},
    logging,
    output::{format_issues, OutputFormat},
    Analyzer,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Arguments for the analyze subcommand
pub struct AnalyzeArgs {
    /// Paths to analyze
    pub paths: Vec<PathBuf>,
    /// PHPStan config file
    pub configuration: Option<PathBuf>,
    /// Analysis level (0-10)
    pub level: Option<u8>,
    /// Output format: raw, json, table, github
    pub error_format: String,
    /// Generate baseline
    pub generate_baseline: Option<PathBuf>,
    /// Use baseline file
    pub baseline: Option<PathBuf>,
    /// Verbose output
    pub verbose: bool,
    /// PHPStan compatibility mode - exactly matches PHPStan behavior
    pub phpstan_compat: bool,
    /// Debug log file path (enables detailed logging)
    pub debug_log: Option<PathBuf>,
    /// Ignore config files
    pub no_config: bool,
    /// Ignore baseline counts (match patterns unlimited times)
    pub ignore_baseline_counts: bool,
}

/// Run the analyze subcommand
pub fn run_analyze(args: AnalyzeArgs) -> Result<ExitCode> {
    // Initialize logging if debug-log is specified
    if let Some(log_path) = &args.debug_log {
        match logging::init_logger(Some(log_path)) {
            Ok(path) => {
                if args.verbose {
                    println!("{}: Debug log writing to {}", "Debug".bold(), path.display());
                }
            }
            Err(e) => {
                eprintln!("{}: Failed to initialize debug log: {}", "Warning".yellow(), e);
            }
        }
    }

    // Load PHPStan configuration
    let mut config = load_config(&args)?;

    // Apply phpstan-compat mode if requested
    if args.phpstan_compat {
        config.phpstan_compat = true;
    }

    if args.verbose {
        println!("{}: {}", "Analysis level".bold(), config.level);
        if !config.paths.is_empty() {
            println!(
                "{}: {}",
                "Configured paths".bold(),
                config.paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
            );
        }
    }

    // Create analyzer
    let mut analyzer = Analyzer::new(config);

    // Override level from CLI if specified
    if let Some(level) = args.level {
        analyzer.set_level(Level::from_u8(level));
    }

    // Determine paths to analyze
    let paths_to_analyze: Vec<&Path> = if args.paths.is_empty() {
        // Use paths from config
        analyzer.config().paths.iter().map(|p| p.as_path()).collect()
    } else {
        args.paths.iter().map(|p| p.as_path()).collect()
    };

    if paths_to_analyze.is_empty() {
        eprintln!("{}: No paths specified for analysis", "Error".red());
        eprintln!("Specify paths on command line or in phpstan.neon configuration file");
        return Ok(ExitCode::from(1));
    }

    if args.verbose {
        println!(
            "{}: {}",
            "Analyzing".bold(),
            paths_to_analyze.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
        );
        println!();
    }

    // Run analysis
    let mut issues = analyzer.analyze_paths(&paths_to_analyze)?;

    // Normalize identifiers for PHPStan compatibility
    if analyzer.config().phpstan_compat {
        issues.normalize_identifiers();
    }

    // Apply baseline filtering from explicit --baseline flag
    if let Some(baseline_path) = &args.baseline {
        if std::env::var("RUSTOR_DEBUG").is_ok() {
            eprintln!("[CLI] Baseline path specified: {}", baseline_path.display());
            eprintln!("[CLI] Baseline path exists: {}", baseline_path.exists());
        }
        if baseline_path.exists() {
            let baseline = Baseline::load(baseline_path)?;
            if std::env::var("RUSTOR_DEBUG").is_ok() {
                eprintln!("[CLI] Loaded {} baseline entries", baseline.len());
                eprintln!("[CLI] Filtering {} issues (ignore_counts={})", issues.len(), args.ignore_baseline_counts);
            }
            issues = baseline.filter_with_options(issues, args.ignore_baseline_counts);
            if std::env::var("RUSTOR_DEBUG").is_ok() {
                eprintln!("[CLI] After filtering: {} issues", issues.len());
            }
            if args.verbose {
                println!("{}: Applied baseline from {}", "Info".bold(), baseline_path.display());
            }
        } else {
            eprintln!("{}: Baseline file not found: {}", "Warning".yellow(), baseline_path.display());
        }
    }
    // Also apply baseline filtering from config's ignoreErrors (from includes)
    else if !analyzer.config().ignore_errors.is_empty() {
        // Convert config ignoreErrors to baseline entries
        let mut baseline = Baseline::new();
        for ignore_error in &analyzer.config().ignore_errors {
            // Create a baseline entry from the ignore error
            // If no path is specified, it applies to all files (use empty string)
            let path = ignore_error.path.clone().unwrap_or_else(|| String::from(""));
            let count = ignore_error.count.unwrap_or(usize::MAX);

            let entry = rustor_analyze::baseline::BaselineEntry::new(
                ignore_error.message.clone(),
                count,
                path,
                ignore_error.identifier.clone(),
            );
            baseline.entries.push(entry);
        }

        let before_count = issues.len();
        issues = baseline.filter_with_options(issues, args.ignore_baseline_counts);
        let after_count = issues.len();
        let filtered = before_count - after_count;

        if args.verbose {
            println!("{}: Applied {} ignoreErrors from config (filtered {} errors)",
                     "Info".bold(), analyzer.config().ignore_errors.len(), filtered);
        }
    }

    // Generate baseline if requested
    if let Some(baseline_output) = &args.generate_baseline {
        let baseline = Baseline::generate(&issues);
        baseline.save(baseline_output)?;
        println!(
            "{}: Generated baseline with {} entries to {}",
            "Done".green(),
            baseline.entries.len(),
            baseline_output.display()
        );
        return Ok(ExitCode::SUCCESS);
    }

    // Determine output format
    let format = OutputFormat::from_str(&args.error_format).unwrap_or(OutputFormat::Table);

    // Format and print output
    let output = format_issues(&issues, format);
    print!("{}", output);

    // Determine exit code
    if issues.error_count() > 0 {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

/// Load PHPStan configuration
fn load_config(args: &AnalyzeArgs) -> Result<PhpStanConfig> {
    // If --no-config is specified, use defaults
    if args.no_config {
        if args.verbose {
            println!("{}: Ignoring config files (--no-config)", "Info".bold());
        }
        return Ok(PhpStanConfig::default());
    }

    // If config path is specified, use it
    if let Some(config_path) = &args.configuration {
        if !config_path.exists() {
            anyhow::bail!("Configuration file not found: {}", config_path.display());
        }
        return Ok(PhpStanConfig::load(config_path)?);
    }

    // Try to find phpstan.neon or phpstan.neon.dist in current directory
    let current_dir = std::env::current_dir()?;
    if let Some(config_path) = PhpStanConfig::find_config(&current_dir) {
        if args.verbose {
            println!("{}: {}", "Using config".bold(), config_path.display());
        }
        return Ok(PhpStanConfig::load(&config_path)?);
    }

    // No config found, use defaults
    if args.verbose {
        println!("{}: Using default configuration", "Info".bold());
    }
    Ok(PhpStanConfig::default())
}

/// Parse arguments for analyze subcommand from command line
pub fn parse_analyze_args(args: &[String]) -> Result<AnalyzeArgs> {
    let mut paths = Vec::new();
    let mut configuration = None;
    let mut level = None;
    let mut error_format = "table".to_string();
    let mut generate_baseline = None;
    let mut baseline = None;
    let mut verbose = false;
    let mut phpstan_compat = false;
    let mut debug_log = None;
    let mut no_config = false;
    let mut ignore_baseline_counts = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if arg == "--no-config" {
            no_config = true;
        } else if arg == "-c" || arg == "--configuration" {
            i += 1;
            if i < args.len() {
                configuration = Some(PathBuf::from(&args[i]));
            }
        } else if arg == "-l" || arg == "--level" {
            i += 1;
            if i >= args.len() {
                anyhow::bail!("--level requires a value (0-10 or 'max')");
            }

            // Handle special "max" value
            let level_value = &args[i];
            if level_value == "max" {
                level = Some(10);
            } else {
                match level_value.parse::<u8>() {
                    Ok(l) if l <= 10 => level = Some(l),
                    Ok(l) => anyhow::bail!("Invalid level: {}. Level must be between 0 and 10", l),
                    Err(_) => anyhow::bail!("Invalid level: '{}'. Expected a number 0-10 or 'max'", level_value),
                }
            }
        } else if let Some(level_str) = arg.strip_prefix("--level=") {
            // Handle --level=5 format
            if level_str == "max" {
                level = Some(10);
            } else {
                match level_str.parse::<u8>() {
                    Ok(l) if l <= 10 => level = Some(l),
                    Ok(l) => anyhow::bail!("Invalid level: {}. Level must be between 0 and 10", l),
                    Err(_) => anyhow::bail!("Invalid level: '{}'. Expected a number 0-10 or 'max'", level_str),
                }
            }
        } else if let Some(level_str) = arg.strip_prefix("-l=") {
            // Handle -l=5 format
            if level_str == "max" {
                level = Some(10);
            } else {
                match level_str.parse::<u8>() {
                    Ok(l) if l <= 10 => level = Some(l),
                    Ok(l) => anyhow::bail!("Invalid level: {}. Level must be between 0 and 10", l),
                    Err(_) => anyhow::bail!("Invalid level: '{}'. Expected a number 0-10 or 'max'", level_str),
                }
            }
        } else if arg == "--error-format" {
            i += 1;
            if i < args.len() {
                error_format = args[i].clone();
            }
        } else if let Some(format) = arg.strip_prefix("--error-format=") {
            error_format = format.to_string();
        } else if arg == "--generate-baseline" {
            i += 1;
            if i < args.len() {
                generate_baseline = Some(PathBuf::from(&args[i]));
            } else {
                generate_baseline = Some(PathBuf::from("phpstan-baseline.neon"));
            }
        } else if arg == "--baseline" {
            i += 1;
            if i < args.len() {
                baseline = Some(PathBuf::from(&args[i]));
            }
        } else if arg == "-v" || arg == "--verbose" {
            verbose = true;
        } else if arg == "--phpstan-compat" {
            phpstan_compat = true;
        } else if arg == "--ignore-baseline-counts" {
            ignore_baseline_counts = true;
        } else if arg == "--debug-log" {
            i += 1;
            if i < args.len() {
                debug_log = Some(PathBuf::from(&args[i]));
            } else {
                // Default path in /tmp
                debug_log = Some(PathBuf::from("/tmp/rustor-analyze.log"));
            }
        } else if let Some(path) = arg.strip_prefix("--debug-log=") {
            debug_log = Some(PathBuf::from(path));
        } else if arg == "-h" || arg == "--help" {
            print_analyze_help();
            std::process::exit(0);
        } else if !arg.starts_with('-') {
            paths.push(PathBuf::from(arg));
        } else {
            // Unknown flag
            anyhow::bail!("Unknown option: '{}'. Use --help to see available options", arg);
        }

        i += 1;
    }

    Ok(AnalyzeArgs {
        paths,
        configuration,
        level,
        error_format,
        generate_baseline,
        baseline,
        verbose,
        phpstan_compat,
        debug_log,
        no_config,
        ignore_baseline_counts,
    })
}

/// Print help for analyze subcommand
pub fn print_analyze_help() {
    println!("{}  PHPStan-compatible static analysis", "rustor analyze".bold());
    println!();
    println!("{}", "USAGE:".bold());
    println!("    rustor analyze [OPTIONS] [PATHS]...");
    println!();
    println!("{}", "ARGS:".bold());
    println!("    <PATHS>...    Paths to analyze (files or directories)");
    println!();
    println!("{}", "OPTIONS:".bold());
    println!("    -c, --configuration <FILE>    PHPStan config file (phpstan.neon)");
    println!("    -l, --level <LEVEL>           Analysis level (0-10, max)");
    println!("        --error-format <FORMAT>   Output format: raw, json, table, github");
    println!("        --generate-baseline <FILE>  Generate baseline file");
    println!("        --baseline <FILE>         Use baseline file to filter issues");
    println!("        --ignore-baseline-counts  Ignore baseline counts (match patterns unlimited times)");
    println!("        --no-config               Ignore config files");
    println!("        --phpstan-compat          PHPStan exact compatibility mode");
    println!("        --debug-log [FILE]        Enable debug logging (default: /tmp/rustor-analyze.log)");
    println!("    -v, --verbose                 Verbose output");
    println!("    -h, --help                    Print help");
    println!();
    println!("{}", "EXAMPLES:".bold());
    println!("    rustor analyze");
    println!("    rustor analyze src/ tests/ --level 5");
    println!("    rustor analyze -c phpstan.neon --error-format json");
    println!("    rustor analyze --generate-baseline baseline.neon");
    println!("    rustor analyze --phpstan-compat --level 1");
    println!("    rustor analyze --debug-log /tmp/my-analyze.log");
}

/// Check if we should run the analyze subcommand
pub fn should_run_analyze(args: &[String]) -> bool {
    args.first().map(|s| s == "analyze").unwrap_or(false)
}
