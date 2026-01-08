//! Report generation for Rector import analysis
//!
//! This module generates compatibility reports showing which Rector rules
//! can be auto-generated and which need manual implementation.

use crate::{ImportResult, RectorRule, RulePattern};
use colored::Colorize;
use std::collections::HashMap;
use std::io::Write;

/// Report format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReportFormat {
    /// Human-readable terminal output
    Terminal,
    /// Markdown format
    Markdown,
    /// JSON format
    Json,
}

/// Generate a compatibility report
pub fn generate_report<W: Write>(
    result: &ImportResult,
    format: ReportFormat,
    writer: &mut W,
) -> std::io::Result<()> {
    match format {
        ReportFormat::Terminal => generate_terminal_report(result, writer),
        ReportFormat::Markdown => generate_markdown_report(result, writer),
        ReportFormat::Json => generate_json_report(result, writer),
    }
}

/// Generate terminal-formatted report
fn generate_terminal_report<W: Write>(result: &ImportResult, writer: &mut W) -> std::io::Result<()> {
    writeln!(writer, "\n{}", "═".repeat(60).blue())?;
    writeln!(writer, "{}", "  Rector Rule Import Report".bold())?;
    writeln!(writer, "{}\n", "═".repeat(60).blue())?;

    // Summary
    let total = result.rules.len();
    let auto_gen = result.auto_generatable_count();
    let manual = total - auto_gen;
    let failed = result.failed.len();

    writeln!(writer, "{}", "Summary".bold().underline())?;
    writeln!(writer, "  Total rules found:     {}", total.to_string().green())?;
    writeln!(writer, "  Auto-generatable:      {}", auto_gen.to_string().green())?;
    writeln!(writer, "  Needs manual review:   {}", manual.to_string().yellow())?;
    writeln!(writer, "  Failed to parse:       {}", failed.to_string().red())?;
    writeln!(writer)?;

    // Coverage percentage
    if total > 0 {
        let coverage = (auto_gen as f64 / total as f64) * 100.0;
        let bar_width = 40;
        let filled = (coverage / 100.0 * bar_width as f64) as usize;
        let bar = format!(
            "[{}{}]",
            "█".repeat(filled).green(),
            "░".repeat(bar_width - filled).dimmed()
        );
        writeln!(writer, "  Auto-generation coverage: {} {:.1}%", bar, coverage)?;
        writeln!(writer)?;
    }

    // Pattern breakdown
    writeln!(writer, "{}", "Pattern Distribution".bold().underline())?;
    let counts = result.count_by_pattern();
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    for (pattern, count) in sorted {
        let emoji = match *pattern {
            "FunctionRename" | "FunctionAlias" => "✓",
            "FunctionToComparison" | "FunctionToCast" | "FunctionToOperator" => "✓",
            "TernaryToCoalesce" | "ArraySyntaxModern" | "ClosureToArrow" => "✓",
            "Complex" => "○",
            "Unknown" => "✗",
            _ => "?",
        };
        let color_count = if *pattern == "Complex" || *pattern == "Unknown" {
            count.to_string().yellow()
        } else {
            count.to_string().green()
        };
        writeln!(writer, "  {} {:25} {}", emoji, pattern, color_count)?;
    }
    writeln!(writer)?;

    // Category breakdown
    writeln!(writer, "{}", "Category Distribution".bold().underline())?;
    let mut category_counts: HashMap<&str, usize> = HashMap::new();
    for rule in &result.rules {
        *category_counts.entry(&rule.category).or_insert(0) += 1;
    }
    let mut sorted_cats: Vec<_> = category_counts.iter().collect();
    sorted_cats.sort_by(|a, b| b.1.cmp(a.1));

    for (cat, count) in sorted_cats.iter().take(10) {
        writeln!(writer, "  {:25} {}", cat, count)?;
    }
    if sorted_cats.len() > 10 {
        writeln!(writer, "  ... and {} more categories", sorted_cats.len() - 10)?;
    }
    writeln!(writer)?;

    // Auto-generatable rules list
    if auto_gen > 0 {
        writeln!(writer, "{}", "Auto-Generatable Rules".bold().underline())?;
        for rule in result.rules.iter().filter(|r| r.pattern.is_auto_generatable()).take(20) {
            writeln!(
                writer,
                "  {} {} - {}",
                "✓".green(),
                rule.name,
                truncate(&rule.description, 50)
            )?;
        }
        if auto_gen > 20 {
            writeln!(writer, "  ... and {} more rules", auto_gen - 20)?;
        }
        writeln!(writer)?;
    }

    // Rules needing review
    let needs_review: Vec<_> = result.rules.iter()
        .filter(|r| !r.pattern.is_auto_generatable())
        .collect();
    if !needs_review.is_empty() {
        writeln!(writer, "{}", "Rules Needing Manual Review".bold().underline())?;
        for rule in needs_review.iter().take(10) {
            let pattern_hint = match &rule.pattern {
                RulePattern::Complex { hints, .. } => {
                    if hints.is_empty() {
                        "Complex pattern".to_string()
                    } else {
                        hints[0].clone()
                    }
                }
                RulePattern::Unknown => "Unknown pattern".to_string(),
                _ => format!("{}", rule.pattern.type_name()),
            };
            writeln!(
                writer,
                "  {} {} - {}",
                "○".yellow(),
                rule.name,
                pattern_hint.dimmed()
            )?;
        }
        if needs_review.len() > 10 {
            writeln!(writer, "  ... and {} more rules", needs_review.len() - 10)?;
        }
        writeln!(writer)?;
    }

    // Warnings
    if !result.warnings.is_empty() {
        writeln!(writer, "{}", "Warnings".bold().underline())?;
        for warning in result.warnings.iter().take(10) {
            writeln!(writer, "  {} {}", "⚠".yellow(), warning)?;
        }
        if result.warnings.len() > 10 {
            writeln!(writer, "  ... and {} more warnings", result.warnings.len() - 10)?;
        }
        writeln!(writer)?;
    }

    // Failures
    if !result.failed.is_empty() {
        writeln!(writer, "{}", "Failed to Parse".bold().underline())?;
        for (file, error) in result.failed.iter().take(5) {
            writeln!(writer, "  {} {}", "✗".red(), file)?;
            writeln!(writer, "    {}", error.dimmed())?;
        }
        if result.failed.len() > 5 {
            writeln!(writer, "  ... and {} more failures", result.failed.len() - 5)?;
        }
    }

    writeln!(writer, "\n{}", "═".repeat(60).blue())?;

    Ok(())
}

/// Generate Markdown report
fn generate_markdown_report<W: Write>(result: &ImportResult, writer: &mut W) -> std::io::Result<()> {
    writeln!(writer, "# Rector Rule Import Report\n")?;

    // Summary table
    let total = result.rules.len();
    let auto_gen = result.auto_generatable_count();
    let manual = total - auto_gen;
    let coverage = if total > 0 { (auto_gen as f64 / total as f64) * 100.0 } else { 0.0 };

    writeln!(writer, "## Summary\n")?;
    writeln!(writer, "| Metric | Count |")?;
    writeln!(writer, "|--------|-------|")?;
    writeln!(writer, "| Total rules | {} |", total)?;
    writeln!(writer, "| Auto-generatable | {} |", auto_gen)?;
    writeln!(writer, "| Needs review | {} |", manual)?;
    writeln!(writer, "| Failed to parse | {} |", result.failed.len())?;
    writeln!(writer, "| **Coverage** | **{:.1}%** |", coverage)?;
    writeln!(writer)?;

    // Pattern distribution
    writeln!(writer, "## Pattern Distribution\n")?;
    writeln!(writer, "| Pattern | Count | Auto-Gen |")?;
    writeln!(writer, "|---------|-------|----------|")?;
    let counts = result.count_by_pattern();
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (pattern, count) in sorted {
        let auto = !matches!(*pattern, "Complex" | "Unknown");
        writeln!(writer, "| {} | {} | {} |", pattern, count, if auto { "✓" } else { "✗" })?;
    }
    writeln!(writer)?;

    // Auto-generatable rules
    writeln!(writer, "## Auto-Generatable Rules\n")?;
    for rule in result.rules.iter().filter(|r| r.pattern.is_auto_generatable()) {
        writeln!(writer, "- **{}**: {}", rule.name, rule.description)?;
    }
    writeln!(writer)?;

    // Rules needing review
    let needs_review: Vec<_> = result.rules.iter()
        .filter(|r| !r.pattern.is_auto_generatable())
        .collect();
    if !needs_review.is_empty() {
        writeln!(writer, "## Rules Needing Manual Review\n")?;
        for rule in needs_review {
            writeln!(writer, "### {}\n", rule.name)?;
            writeln!(writer, "- **Category**: {}", rule.category)?;
            writeln!(writer, "- **Description**: {}", rule.description)?;
            writeln!(writer, "- **Node Types**: {}", rule.node_types.join(", "))?;
            if let RulePattern::Complex { hints, .. } = &rule.pattern {
                if !hints.is_empty() {
                    writeln!(writer, "- **Hints**:")?;
                    for hint in hints {
                        writeln!(writer, "  - {}", hint)?;
                    }
                }
            }
            writeln!(writer)?;
        }
    }

    // Failures
    if !result.failed.is_empty() {
        writeln!(writer, "## Failed to Parse\n")?;
        for (file, error) in &result.failed {
            writeln!(writer, "- `{}`: {}", file, error)?;
        }
    }

    Ok(())
}

/// Generate JSON report
fn generate_json_report<W: Write>(result: &ImportResult, writer: &mut W) -> std::io::Result<()> {
    let report = serde_json::json!({
        "summary": {
            "total_rules": result.rules.len(),
            "auto_generatable": result.auto_generatable_count(),
            "needs_review": result.rules.len() - result.auto_generatable_count(),
            "failed": result.failed.len(),
            "warnings": result.warnings.len(),
        },
        "pattern_counts": result.count_by_pattern(),
        "rules": result.rules.iter().map(|r| {
            serde_json::json!({
                "name": r.name,
                "category": r.category,
                "description": r.description,
                "pattern": r.pattern.type_name(),
                "auto_generatable": r.pattern.is_auto_generatable(),
                "min_php_version": r.min_php_version,
                "source_file": r.source_file,
            })
        }).collect::<Vec<_>>(),
        "failed": result.failed.iter().map(|(f, e)| {
            serde_json::json!({ "file": f, "error": e })
        }).collect::<Vec<_>>(),
        "warnings": result.warnings,
    });

    writeln!(writer, "{}", serde_json::to_string_pretty(&report).unwrap())?;
    Ok(())
}

/// Truncate a string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Print a quick summary to stdout
pub fn print_summary(result: &ImportResult) {
    let total = result.rules.len();
    let auto_gen = result.auto_generatable_count();

    println!(
        "\n{} {} rules found, {} ({:.0}%) can be auto-generated\n",
        "→".blue(),
        total,
        auto_gen,
        if total > 0 { auto_gen as f64 / total as f64 * 100.0 } else { 0.0 }
    );
}

/// List all auto-generatable rules
pub fn list_auto_generatable(result: &ImportResult) -> Vec<&RectorRule> {
    result.rules.iter()
        .filter(|r| r.pattern.is_auto_generatable())
        .collect()
}

/// List all rules needing manual review
pub fn list_needs_review(result: &ImportResult) -> Vec<&RectorRule> {
    result.rules.iter()
        .filter(|r| !r.pattern.is_auto_generatable())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello world", 20), "hello world");
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_generate_terminal_report() {
        let result = ImportResult::new();
        let mut output = Vec::new();
        generate_report(&result, ReportFormat::Terminal, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Rector Rule Import Report"));
    }

    #[test]
    fn test_generate_json_report() {
        let result = ImportResult::new();
        let mut output = Vec::new();
        generate_report(&result, ReportFormat::Json, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();
        assert!(parsed.get("summary").is_some());
    }
}
