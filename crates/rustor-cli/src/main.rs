//! rustor CLI - PHP refactoring tool
//!
//! Phase 0: Proof of concept with array_push rule

use anyhow::{Context, Result};
use bumpalo::Bump;
use clap::Parser;
use colored::*;
use mago_database::file::FileId;
use std::path::{Path, PathBuf};

use rustor_core::apply_edits;
use rustor_rules::check_array_push;

#[derive(Parser)]
#[command(name = "rustor")]
#[command(version = "0.1.0")]
#[command(about = "A Rust-based PHP refactoring tool")]
#[command(author = "rustor contributors")]
struct Cli {
    /// Files or directories to process
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Show changes without applying them
    #[arg(long, short = 'n')]
    dry_run: bool,

    /// Show verbose output
    #[arg(long, short = 'v')]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut total_files = 0;
    let mut files_with_changes = 0;
    let mut total_edits = 0;

    for path in &cli.paths {
        if path.is_file() {
            let (changes, edits) = process_file(path, cli.dry_run, cli.verbose)?;
            total_files += 1;
            if changes {
                files_with_changes += 1;
            }
            total_edits += edits;
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "php"))
            {
                let (changes, edits) = process_file(entry.path(), cli.dry_run, cli.verbose)?;
                total_files += 1;
                if changes {
                    files_with_changes += 1;
                }
                total_edits += edits;
            }
        } else {
            eprintln!(
                "{}: Path does not exist: {}",
                "Warning".yellow(),
                path.display()
            );
        }
    }

    // Print summary
    println!();
    println!("{}", "Summary".bold().underline());
    println!("  Files processed: {}", total_files);
    println!("  Files with changes: {}", files_with_changes);
    println!("  Total edits: {}", total_edits);

    if cli.dry_run && total_edits > 0 {
        println!();
        println!("{}", "Run without --dry-run to apply changes".yellow());
    }

    Ok(())
}

fn process_file(path: &Path, dry_run: bool, verbose: bool) -> Result<(bool, usize)> {
    let source_code = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    // Create arena allocator and file ID for mago
    let arena = Bump::new();
    let file_id = FileId::new(path.to_string_lossy().as_ref());

    // Parse the PHP file
    let (program, parse_error) = mago_syntax::parser::parse_file_content(&arena, file_id, &source_code);

    // Check for parse errors
    if let Some(error) = parse_error {
        if verbose {
            eprintln!(
                "{}: Parse errors in {}, skipping",
                "Warning".yellow(),
                path.display()
            );
            eprintln!("  {}", error);
        }
        return Ok((false, 0));
    }

    // Apply refactoring rules
    let edits = check_array_push(program, &source_code);

    if edits.is_empty() {
        if verbose {
            println!("{}: No changes needed", path.display());
        }
        return Ok((false, 0));
    }

    let edit_count = edits.len();

    // Print file header
    println!("{}", path.display().to_string().bold());

    // Apply edits
    let new_source = apply_edits(&source_code, &edits)
        .with_context(|| format!("Failed to apply edits to {}", path.display()))?;

    if dry_run {
        // Show diff
        print_diff(&source_code, &new_source);

        // List changes
        println!();
        for edit in &edits {
            println!("  {} {}", "->".green(), edit.message);
        }
    } else {
        // Write changes
        std::fs::write(path, &new_source)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;

        println!("  {} Applied {} change(s)", "OK".green(), edit_count);
    }

    println!();

    Ok((true, edit_count))
}

fn print_diff(old: &str, new: &str) {
    for diff_result in diff::lines(old, new) {
        match diff_result {
            diff::Result::Left(l) => {
                println!("  {}", format!("- {}", l).red());
            }
            diff::Result::Right(r) => {
                println!("  {}", format!("+ {}", r).green());
            }
            diff::Result::Both(_, _) => {
                // Skip unchanged lines for cleaner output
            }
        }
    }
}
