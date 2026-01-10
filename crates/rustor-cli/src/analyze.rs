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
    /// Analysis level (0-9)
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

    // Apply baseline filtering if specified
    if let Some(baseline_path) = &args.baseline {
        if baseline_path.exists() {
            let baseline = Baseline::load(baseline_path)?;
            issues = baseline.filter(issues);
            if args.verbose {
                println!("{}: Applied baseline from {}", "Info".bold(), baseline_path.display());
            }
        } else {
            eprintln!("{}: Baseline file not found: {}", "Warning".yellow(), baseline_path.display());
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

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if arg == "-c" || arg == "--configuration" {
            i += 1;
            if i < args.len() {
                configuration = Some(PathBuf::from(&args[i]));
            }
        } else if arg == "-l" || arg == "--level" {
            i += 1;
            if i < args.len() {
                level = args[i].parse().ok();
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
    println!("    -l, --level <LEVEL>           Analysis level (0-9, max)");
    println!("        --error-format <FORMAT>   Output format: raw, json, table, github");
    println!("        --generate-baseline <FILE>  Generate baseline file");
    println!("        --baseline <FILE>         Use baseline file to filter issues");
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
