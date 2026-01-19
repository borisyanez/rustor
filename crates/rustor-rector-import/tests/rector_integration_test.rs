//! Integration tests for importing Rector rules from the actual Rector repository
//!
//! These tests validate the import process against the real Rector codebase
//! located at ~/PhpProjects/rector

use rustor_rector_import::{RectorRule, RulePattern};
use rustor_rector_import::rule_extractor::{extract_rules_from_repo, extract_rule_from_file};
use std::collections::HashMap;
use std::path::Path;

const RECTOR_REPO_PATH: &str = "/Users/borisyv/PhpProjects/rector";

/// Test that we can successfully import rules from the Rector repository
#[test]
fn test_import_from_rector_repo() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        eprintln!("Skipping test: Rector repo not found at {}", RECTOR_REPO_PATH);
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    // Print summary
    println!("\n=== Rector Import Summary ===");
    println!("Total rules parsed: {}", result.rules.len());
    println!("Failed to parse: {}", result.failed.len());
    println!("Auto-generatable: {}", result.auto_generatable_count());
    println!("Warnings: {}", result.warnings.len());

    // Pattern distribution
    println!("\n=== Pattern Distribution ===");
    let counts = result.count_by_pattern();
    let mut sorted_counts: Vec<_> = counts.iter().collect();
    sorted_counts.sort_by(|a, b| b.1.cmp(a.1));
    for (pattern, count) in sorted_counts {
        println!("  {}: {}", pattern, count);
    }

    // We should have imported a significant number of rules
    assert!(
        result.rules.len() > 100,
        "Expected to import >100 rules, got {}",
        result.rules.len()
    );

    // Auto-generatable should be a reasonable percentage
    let auto_gen_pct = (result.auto_generatable_count() as f64 / result.rules.len() as f64) * 100.0;
    println!("\nAuto-generatable percentage: {:.1}%", auto_gen_pct);
}

/// Test specific rule categories and their patterns
#[test]
fn test_php_version_rules() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    // Group rules by category
    let mut by_category: HashMap<String, Vec<&RectorRule>> = HashMap::new();
    for rule in &result.rules {
        by_category
            .entry(rule.category.clone())
            .or_default()
            .push(rule);
    }

    println!("\n=== Rules by Category ===");
    let mut sorted_cats: Vec<_> = by_category.iter().collect();
    sorted_cats.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    for (cat, rules) in sorted_cats.iter().take(15) {
        println!("  {}: {} rules", cat, rules.len());
    }

    // Check that we have rules for various PHP versions
    let php_cats = ["Php70", "Php71", "Php72", "Php73", "Php74", "Php80", "Php81", "Php82", "Php83", "Php84"];
    println!("\n=== PHP Version Rule Coverage ===");
    for cat in php_cats {
        let count = by_category.get(cat).map(|v| v.len()).unwrap_or(0);
        println!("  {}: {} rules", cat, count);
    }
}

/// Test known function rename patterns
#[test]
fn test_function_rename_detection() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    // Find rules with FunctionRename or FunctionAlias patterns
    let rename_rules: Vec<_> = result
        .rules
        .iter()
        .filter(|r| {
            matches!(
                &r.pattern,
                RulePattern::FunctionRename { .. } | RulePattern::FunctionAlias { .. }
            )
        })
        .collect();

    println!("\n=== Function Rename/Alias Rules ===");
    for rule in rename_rules.iter().take(20) {
        match &rule.pattern {
            RulePattern::FunctionRename { from, to } => {
                println!("  {} -> {} ({})", from, to, rule.name);
            }
            RulePattern::FunctionAlias { from, to } => {
                println!("  {} -> {} [alias] ({})", from, to, rule.name);
            }
            _ => {}
        }
    }
    println!("  ... and {} more", rename_rules.len().saturating_sub(20));
}

/// Test function to comparison patterns (is_null -> === null)
#[test]
fn test_function_to_comparison_detection() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    let comparison_rules: Vec<_> = result
        .rules
        .iter()
        .filter(|r| matches!(&r.pattern, RulePattern::FunctionToComparison { .. }))
        .collect();

    println!("\n=== Function to Comparison Rules ===");
    for rule in &comparison_rules {
        if let RulePattern::FunctionToComparison {
            func,
            operator,
            compare_value,
        } = &rule.pattern
        {
            println!("  {}($x) -> $x {} {} ({})", func, operator, compare_value, rule.name);
        }
    }
}

/// Test function to cast patterns (strval -> (string))
#[test]
fn test_function_to_cast_detection() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    let cast_rules: Vec<_> = result
        .rules
        .iter()
        .filter(|r| matches!(&r.pattern, RulePattern::FunctionToCast { .. }))
        .collect();

    println!("\n=== Function to Cast Rules ===");
    for rule in &cast_rules {
        if let RulePattern::FunctionToCast { func, cast_type } = &rule.pattern {
            println!("  {}($x) -> ({}) $x ({})", func, cast_type, rule.name);
        }
    }
}

/// Test string function patterns (strpos !== false -> str_contains)
#[test]
fn test_string_function_detection() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    let str_rules: Vec<_> = result
        .rules
        .iter()
        .filter(|r| {
            matches!(
                &r.pattern,
                RulePattern::StrContains | RulePattern::StrStartsWith | RulePattern::StrEndsWith
            )
        })
        .collect();

    println!("\n=== String Function Rules (PHP 8.0+) ===");
    for rule in &str_rules {
        println!("  {} - {}", rule.pattern.type_name(), rule.name);
    }
}

/// Test that code samples are extracted from rules
#[test]
fn test_code_samples_extraction() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    // Count rules with code samples
    let with_samples = result
        .rules
        .iter()
        .filter(|r| !r.before_code.is_empty() && !r.after_code.is_empty())
        .count();

    let total = result.rules.len();
    let pct = (with_samples as f64 / total as f64) * 100.0;

    println!("\n=== Code Samples Extraction ===");
    println!("Rules with code samples: {} / {} ({:.1}%)", with_samples, total, pct);

    // We should extract code samples from most rules
    assert!(
        pct > 80.0,
        "Expected >80% of rules to have code samples, got {:.1}%",
        pct
    );
}

/// Test specific known rules to validate pattern detection
#[test]
fn test_known_rules_patterns() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    // Test specific rule files
    let test_cases = [
        (
            "rules/Php74/Rector/FuncCall/FilterVarToAddSlashesRector.php",
            "filter_var",
            "Complex", // This is actually complex because it checks the second arg
        ),
        (
            "rules/Php74/Rector/FuncCall/RestoreIncludePathToIniRestoreRector.php",
            "restore_include_path",
            "FunctionNoArgsToFunction",
        ),
    ];

    println!("\n=== Known Rules Pattern Validation ===");
    for (path, expected_func, expected_pattern) in test_cases {
        let full_path = repo_path.join(path);
        if !full_path.exists() {
            println!("  [SKIP] {} - file not found", path);
            continue;
        }

        match extract_rule_from_file(&full_path) {
            Ok(Some(rule)) => {
                let pattern_name = rule.pattern.type_name();
                let matches = pattern_name == expected_pattern;
                let status = if matches { "OK" } else { "MISMATCH" };
                println!(
                    "  [{}] {} -> expected {}, got {}",
                    status, rule.name, expected_pattern, pattern_name
                );

                // Check if the function name is mentioned in description or code
                let has_func = rule.before_code.contains(expected_func)
                    || rule.description.to_lowercase().contains(expected_func);
                if has_func {
                    println!("       Function '{}' found in code/description", expected_func);
                }
            }
            Ok(None) => println!("  [SKIP] {} - not a Rector rule", path),
            Err(e) => println!("  [ERR] {} - {}", path, e),
        }
    }
}

/// Test complex rules detection
#[test]
fn test_complex_rules_analysis() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    let complex_rules: Vec<_> = result
        .rules
        .iter()
        .filter(|r| matches!(&r.pattern, RulePattern::Complex { .. }))
        .collect();

    println!("\n=== Complex Rules Analysis ===");
    println!("Total complex rules: {}", complex_rules.len());

    // Sample some complex rules with their hints
    println!("\nSample complex rules with hints:");
    for rule in complex_rules.iter().take(10) {
        if let RulePattern::Complex { hints, .. } = &rule.pattern {
            println!("  {} ({}):", rule.name, rule.category);
            for hint in hints.iter().take(3) {
                println!("    - {}", hint);
            }
        }
    }
}

/// Test import performance
#[test]
fn test_import_performance() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let start = std::time::Instant::now();
    let result = extract_rules_from_repo(repo_path);
    let duration = start.elapsed();

    println!("\n=== Import Performance ===");
    println!("Time to import {} rules: {:?}", result.rules.len(), duration);
    println!(
        "Average per rule: {:?}",
        duration / result.rules.len() as u32
    );

    // Import should complete in reasonable time (AST parsing is slow)
    assert!(
        duration.as_secs() < 120,
        "Import took too long: {:?}",
        duration
    );
}

/// Analyze unknown patterns to identify improvement opportunities
#[test]
fn test_analyze_unknown_patterns() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    let unknown_rules: Vec<_> = result
        .rules
        .iter()
        .filter(|r| matches!(&r.pattern, RulePattern::Unknown))
        .collect();

    println!("\n=== Unknown Pattern Analysis ===");
    println!("Total unknown: {}", unknown_rules.len());

    // Categorize by node types
    let mut by_node_type: HashMap<String, usize> = HashMap::new();
    for rule in &unknown_rules {
        for node_type in &rule.node_types {
            *by_node_type.entry(node_type.clone()).or_insert(0) += 1;
        }
    }

    println!("\nUnknown rules by node type:");
    let mut sorted: Vec<_> = by_node_type.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (node_type, count) in sorted.iter().take(15) {
        println!("  {:30} {:>4}", node_type, count);
    }

    // Sample unknown rules by category
    println!("\nSample unknown rules:");
    let mut by_cat: HashMap<&str, Vec<&RectorRule>> = HashMap::new();
    for rule in &unknown_rules {
        by_cat.entry(&rule.category).or_default().push(rule);
    }

    for (cat, rules) in by_cat.iter().take(5) {
        println!("\n  {}:", cat);
        for rule in rules.iter().take(3) {
            println!("    - {} (nodes: {:?})", rule.name, rule.node_types);
        }
    }
}

/// Generate a compatibility report
#[test]
fn test_generate_compatibility_report() {
    let repo_path = Path::new(RECTOR_REPO_PATH);
    if !repo_path.exists() {
        return;
    }

    let result = extract_rules_from_repo(repo_path);

    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║           RECTOR IMPORT COMPATIBILITY REPORT                     ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║ Total Rector rules found:           {:>5}                        ║", result.rules.len());
    println!("║ Failed to parse:                    {:>5}                        ║", result.failed.len());
    println!("║ Auto-generatable (YAML/Rust):       {:>5}                        ║", result.auto_generatable_count());
    println!("║ Complex (need manual impl):         {:>5}                        ║",
        result.rules.iter().filter(|r| matches!(r.pattern, RulePattern::Complex { .. })).count());
    println!("║ Unknown pattern:                    {:>5}                        ║",
        result.rules.iter().filter(|r| matches!(r.pattern, RulePattern::Unknown)).count());
    println!("╠══════════════════════════════════════════════════════════════════╣");

    let total = result.rules.len() as f64;
    let auto_pct = (result.auto_generatable_count() as f64 / total) * 100.0;
    println!("║ Auto-generation coverage:           {:>5.1}%                       ║", auto_pct);
    println!("╚══════════════════════════════════════════════════════════════════╝");

    // Detailed pattern breakdown
    println!("\nPattern Breakdown:");
    let counts = result.count_by_pattern();
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (pattern, count) in sorted {
        let pct = (*count as f64 / total) * 100.0;
        println!("  {:30} {:>4} ({:>5.1}%)", pattern, count, pct);
    }

    // Rules with descriptions
    let with_desc = result.rules.iter().filter(|r| !r.description.is_empty()).count();
    println!("\nMetadata Extraction:");
    println!("  Rules with description: {} ({:.1}%)", with_desc, (with_desc as f64 / total) * 100.0);

    let with_samples = result.rules.iter()
        .filter(|r| !r.before_code.is_empty() && !r.after_code.is_empty())
        .count();
    println!("  Rules with code samples: {} ({:.1}%)", with_samples, (with_samples as f64 / total) * 100.0);

    let with_version = result.rules.iter().filter(|r| r.min_php_version.is_some()).count();
    println!("  Rules with PHP version: {} ({:.1}%)", with_version, (with_version as f64 / total) * 100.0);
}
