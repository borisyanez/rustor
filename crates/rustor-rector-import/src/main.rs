//! rustor-import-rector CLI
//!
//! Command-line tool to import Rector PHP rules and generate rustor Rust rules.
//!
//! Usage:
//!   rustor-import-rector --rector-path ./rector --output ./imported/
//!   rustor-import-rector --list-compatible
//!   rustor-import-rector --report > compatibility.md

use clap::{Parser, Subcommand};
use colored::Colorize;
use rustor_rector_import::codegen::CodeGenerator;
use rustor_rector_import::report::{generate_report, print_summary, ReportFormat};
use rustor_rector_import::rule_extractor::{extract_rules_from_category, extract_rules_from_repo};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "rustor-import-rector")]
#[command(about = "Import Rector PHP rules into rustor")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to Rector repository
    #[arg(short = 'r', long, value_name = "PATH")]
    rector_path: Option<PathBuf>,

    /// Output directory for generated rules
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Only import specific category (e.g., CodeQuality, Php80)
    #[arg(short, long)]
    category: Option<String>,

    /// Dry run - don't write files, just show what would be generated
    #[arg(long)]
    dry_run: bool,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List all compatible (auto-generatable) rules
    List {
        /// Path to Rector repository
        #[arg(short = 'r', long)]
        rector_path: PathBuf,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Generate compatibility report
    Report {
        /// Path to Rector repository
        #[arg(short = 'r', long)]
        rector_path: PathBuf,

        /// Report format: terminal, markdown, json
        #[arg(short, long, default_value = "terminal")]
        format: String,

        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate Rust rules from Rector
    Generate {
        /// Path to Rector repository
        #[arg(short = 'r', long)]
        rector_path: PathBuf,

        /// Output directory for generated rules
        #[arg(short, long)]
        output: PathBuf,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Only generate auto-generatable rules (skip complex/unknown)
        #[arg(long)]
        auto_only: bool,

        /// Dry run - don't write files
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate YAML rules from Rector (easier to review and modify)
    GenerateYaml {
        /// Path to Rector repository
        #[arg(short = 'r', long)]
        rector_path: PathBuf,

        /// Output directory for generated YAML rules
        #[arg(short, long)]
        output: PathBuf,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Dry run - don't write files
        #[arg(long)]
        dry_run: bool,

        /// Show generated YAML for review
        #[arg(long)]
        show_yaml: bool,
    },

    /// Analyze a single Rector rule file
    Analyze {
        /// Path to Rector rule file
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::List { rector_path, category }) => {
            cmd_list(&rector_path, category.as_deref())?;
        }
        Some(Commands::Report { rector_path, format, output }) => {
            cmd_report(&rector_path, &format, output.as_deref())?;
        }
        Some(Commands::Generate { rector_path, output, category, auto_only, dry_run }) => {
            cmd_generate(&rector_path, &output, category.as_deref(), auto_only, dry_run)?;
        }
        Some(Commands::GenerateYaml { rector_path, output, category, dry_run, show_yaml }) => {
            cmd_generate_yaml(&rector_path, &output, category.as_deref(), dry_run, show_yaml)?;
        }
        Some(Commands::Analyze { file }) => {
            cmd_analyze(&file)?;
        }
        None => {
            // Default behavior: generate rules if rector_path and output provided
            if let (Some(rector_path), Some(output)) = (cli.rector_path, cli.output) {
                cmd_generate(&rector_path, &output, cli.category.as_deref(), false, cli.dry_run)?;
            } else {
                // Show help
                eprintln!("{}", "Usage: rustor-import-rector <COMMAND>".yellow());
                eprintln!("\nCommands:");
                eprintln!("  {} - List compatible rules", "list".green());
                eprintln!("  {} - Generate compatibility report", "report".green());
                eprintln!("  {} - Generate Rust rules", "generate".green());
                eprintln!("  {} - Analyze a single rule file", "analyze".green());
                eprintln!("\nRun with --help for more information");
            }
        }
    }

    Ok(())
}

/// List compatible rules
fn cmd_list(rector_path: &PathBuf, category: Option<&str>) -> anyhow::Result<()> {
    println!("{}", "Scanning Rector repository...".blue());

    let result = if let Some(cat) = category {
        extract_rules_from_category(rector_path, cat)
    } else {
        extract_rules_from_repo(rector_path)
    };

    let auto_gen: Vec<_> = result.rules.iter()
        .filter(|r| r.pattern.is_auto_generatable())
        .collect();

    println!("\n{} {} auto-generatable rules:\n",
        "Found".green(),
        auto_gen.len().to_string().bold()
    );

    for rule in &auto_gen {
        println!(
            "  {} {} [{}] - {}",
            "✓".green(),
            rule.name.bold(),
            rule.pattern.type_name().dimmed(),
            truncate(&rule.description, 50)
        );
    }

    print_summary(&result);

    Ok(())
}

/// Generate compatibility report
fn cmd_report(rector_path: &PathBuf, format: &str, output: Option<&Path>) -> anyhow::Result<()> {
    println!("{}", "Scanning Rector repository...".blue());

    let result = extract_rules_from_repo(rector_path);

    let report_format = match format.to_lowercase().as_str() {
        "markdown" | "md" => ReportFormat::Markdown,
        "json" => ReportFormat::Json,
        _ => ReportFormat::Terminal,
    };

    if let Some(output_path) = output {
        let mut file = fs::File::create(output_path)?;
        generate_report(&result, report_format, &mut file)?;
        println!("{} Report written to: {}", "✓".green(), output_path.display());
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        generate_report(&result, report_format, &mut handle)?;
    }

    Ok(())
}

/// Generate Rust rules
fn cmd_generate(
    rector_path: &PathBuf,
    output: &PathBuf,
    category: Option<&str>,
    auto_only: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    println!("{}", "Scanning Rector repository...".blue());

    let result = if let Some(cat) = category {
        extract_rules_from_category(rector_path, cat)
    } else {
        extract_rules_from_repo(rector_path)
    };

    println!(
        "{} Found {} rules ({} auto-generatable)",
        "→".blue(),
        result.rules.len(),
        result.auto_generatable_count()
    );

    // Filter rules
    let rules_to_generate: Vec<_> = if auto_only {
        result.rules.iter()
            .filter(|r| r.pattern.is_auto_generatable())
            .collect()
    } else {
        result.rules.iter().collect()
    };

    if rules_to_generate.is_empty() {
        println!("{}", "No rules to generate".yellow());
        return Ok(());
    }

    println!("{} Generating {} rules...", "→".blue(), rules_to_generate.len());

    let generator = CodeGenerator::new();
    let mut generated = Vec::new();
    let mut errors = Vec::new();

    for rule in &rules_to_generate {
        match generator.generate_rule(rule) {
            Ok(gen) => {
                if dry_run {
                    let status = if gen.needs_review {
                        "needs review".yellow()
                    } else {
                        "ready".green()
                    };
                    println!("  {} {} ({})", "•".dimmed(), gen.filename, status);
                }
                generated.push(gen);
            }
            Err(e) => {
                errors.push((rule.name.clone(), e));
            }
        }
    }

    // Summary
    let ready = generated.iter().filter(|g| !g.needs_review).count();
    let needs_review = generated.len() - ready;

    println!("\n{}", "Generation Summary".bold().underline());
    println!("  {} rules ready to use", ready.to_string().green());
    println!("  {} rules need manual review", needs_review.to_string().yellow());
    if !errors.is_empty() {
        println!("  {} rules failed to generate", errors.len().to_string().red());
    }

    if dry_run {
        println!("\n{}", "Dry run - no files written".yellow());
    } else {
        // Write files
        generator.write_rules(&generated, output)
            .map_err(|e| anyhow::anyhow!(e))?;
        println!("\n{} Rules written to: {}", "✓".green(), output.display());
    }

    // Show errors
    if !errors.is_empty() {
        println!("\n{}", "Generation Errors:".red());
        for (name, error) in &errors {
            println!("  {} {}: {}", "✗".red(), name, error);
        }
    }

    Ok(())
}

/// Generate YAML rules
fn cmd_generate_yaml(
    rector_path: &PathBuf,
    output: &PathBuf,
    category: Option<&str>,
    dry_run: bool,
    show_yaml: bool,
) -> anyhow::Result<()> {
    use rustor_rector_import::yaml_codegen::generate_yaml_rule;

    println!("{}", "Scanning Rector repository...".blue());

    let result = if let Some(cat) = category {
        extract_rules_from_category(rector_path, cat)
    } else {
        extract_rules_from_repo(rector_path)
    };

    println!(
        "{} Found {} rules ({} auto-generatable)",
        "→".blue(),
        result.rules.len(),
        result.auto_generatable_count()
    );

    // Only generate auto-generatable rules for YAML
    let rules_to_generate: Vec<_> = result.rules.iter()
        .filter(|r| r.pattern.is_auto_generatable())
        .collect();

    if rules_to_generate.is_empty() {
        println!("{}", "No auto-generatable rules found".yellow());
        return Ok(());
    }

    println!("{} Generating {} YAML rules...", "→".blue(), rules_to_generate.len());

    let mut generated = Vec::new();
    let mut skipped = Vec::new();

    for rule in &rules_to_generate {
        if let Some(yaml) = generate_yaml_rule(rule) {
            let filename = format!("{}.yaml", to_snake_case(&rule.name.replace("Rector", "")));

            if show_yaml {
                println!("\n{} {}", "=".repeat(60).dimmed(), "");
                println!("{}: {}", "File".bold(), filename);
                println!("{}", "=".repeat(60).dimmed());
                println!("{}", yaml);
            } else if dry_run {
                println!("  {} {}", "✓".green(), filename);
            }

            generated.push((filename, yaml, rule.name.clone()));
        } else {
            skipped.push(&rule.name);
        }
    }

    // Summary
    println!("\n{}", "Generation Summary".bold().underline());
    println!("  {} YAML rules generated", generated.len().to_string().green());
    if !skipped.is_empty() {
        println!("  {} rules skipped (complex patterns)", skipped.len().to_string().yellow());
    }

    if dry_run {
        println!("\n{}", "Dry run - no files written".yellow());
    } else if !generated.is_empty() {
        // Create output directory
        fs::create_dir_all(output)?;

        // Write files
        for (filename, yaml, _name) in &generated {
            let path = output.join(filename);
            fs::write(&path, yaml)?;
        }

        println!("\n{} {} rules written to: {}", "✓".green(), generated.len(), output.display());
    }

    Ok(())
}

/// Convert CamelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

/// Analyze a single rule file
fn cmd_analyze(file: &PathBuf) -> anyhow::Result<()> {
    use rustor_rector_import::rule_extractor::extract_rule_from_file;

    println!("{} Analyzing: {}\n", "→".blue(), file.display());

    match extract_rule_from_file(file).map_err(|e| anyhow::anyhow!(e))? {
        Some(rule) => {
            println!("{}", "Rule Information".bold().underline());
            println!("  Name:          {}", rule.name.green());
            println!("  Category:      {}", rule.category);
            println!("  Description:   {}", rule.description);
            println!("  Node Types:    {}", rule.node_types.join(", "));
            println!("  PHP Version:   {}", rule.min_php_version.as_deref().unwrap_or("any"));
            println!("  Configurable:  {}", rule.is_configurable);
            println!();

            println!("{}", "Pattern Analysis".bold().underline());
            println!("  Type:          {}", rule.pattern.type_name());
            println!(
                "  Auto-Gen:      {}",
                if rule.pattern.is_auto_generatable() {
                    "Yes".green()
                } else {
                    "No (needs manual review)".yellow()
                }
            );

            match &rule.pattern {
                rustor_rector_import::RulePattern::FunctionRename { from, to } => {
                    println!("  Transform:     {} → {}", from, to);
                }
                rustor_rector_import::RulePattern::FunctionAlias { from, to } => {
                    println!("  Alias:         {} → {}", from, to);
                }
                rustor_rector_import::RulePattern::FunctionToComparison { func, operator, compare_value } => {
                    println!("  Transform:     {}($x) → $x {} {}", func, operator, compare_value);
                }
                rustor_rector_import::RulePattern::FunctionToCast { func, cast_type } => {
                    println!("  Transform:     {}($x) → ({}) $x", func, cast_type);
                }
                rustor_rector_import::RulePattern::FunctionToOperator { func, operator, .. } => {
                    println!("  Transform:     {}($a, $b) → $a {} $b", func, operator);
                }
                rustor_rector_import::RulePattern::TernaryToCoalesce { condition_func } => {
                    println!("  Transform:     {}($x) ? $x : $d → $x ?? $d", condition_func);
                }
                rustor_rector_import::RulePattern::Complex { hints, .. } => {
                    if !hints.is_empty() {
                        println!("  Hints:");
                        for hint in hints {
                            println!("    - {}", hint);
                        }
                    }
                }
                _ => {}
            }

            if !rule.before_code.is_empty() {
                println!();
                println!("{}", "Code Sample".bold().underline());
                println!("  Before: {}", rule.before_code);
                println!("  After:  {}", rule.after_code);
            }
        }
        None => {
            println!("{}", "Could not extract rule from file".red());
        }
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
