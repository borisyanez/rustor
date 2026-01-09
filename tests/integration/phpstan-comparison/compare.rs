//! PHPStan comparison integration tests
//!
//! These tests compare rustor-analyze output with PHPStan to ensure compatibility.

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Represents an error found by an analyzer
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AnalyzerError {
    line: u32,
    identifier: String,
    message_contains: String,
}

/// Parse PHPStan JSON output into errors
fn parse_phpstan_output(output: &str) -> Vec<AnalyzerError> {
    let mut errors = Vec::new();

    // Parse JSON output
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(files) = json.get("files").and_then(|f| f.as_object()) {
            for (_file, file_data) in files {
                if let Some(messages) = file_data.get("messages").and_then(|m| m.as_array()) {
                    for msg in messages {
                        if let (Some(line), Some(message), identifier) = (
                            msg.get("line").and_then(|l| l.as_u64()),
                            msg.get("message").and_then(|m| m.as_str()),
                            msg.get("identifier").and_then(|i| i.as_str()),
                        ) {
                            errors.push(AnalyzerError {
                                line: line as u32,
                                identifier: identifier.unwrap_or("unknown").to_string(),
                                message_contains: extract_key_phrase(message),
                            });
                        }
                    }
                }
            }
        }
    }

    errors
}

/// Parse rustor JSON output into errors
fn parse_rustor_output(output: &str) -> Vec<AnalyzerError> {
    let mut errors = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(issues) = json.get("issues").and_then(|i| i.as_array()) {
            for issue in issues {
                if let (Some(line), Some(message), identifier) = (
                    issue.get("line").and_then(|l| l.as_u64()),
                    issue.get("message").and_then(|m| m.as_str()),
                    issue.get("identifier").and_then(|i| i.as_str()),
                ) {
                    errors.push(AnalyzerError {
                        line: line as u32,
                        identifier: identifier.unwrap_or("unknown").to_string(),
                        message_contains: extract_key_phrase(message),
                    });
                }
            }
        }
    }

    errors
}

/// Extract a key phrase from an error message for comparison
fn extract_key_phrase(message: &str) -> String {
    // Normalize the message for comparison
    message
        .to_lowercase()
        .replace("instantiated class", "class")
        .replace("not found", "")
        .replace("undefined", "")
        .replace("might not be defined", "")
        .trim()
        .to_string()
}

/// Run PHPStan on a file and return errors
fn run_phpstan(file: &Path, level: u8) -> Result<Vec<AnalyzerError>, String> {
    let phpstan_path = "/Users/boris/PhpProjects/phpstan-src/bin/phpstan";

    let output = Command::new(phpstan_path)
        .args([
            "analyze",
            file.to_str().unwrap(),
            "--level",
            &level.to_string(),
            "--error-format=json",
            "--no-progress",
        ])
        .output()
        .map_err(|e| format!("Failed to run PHPStan: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_phpstan_output(&stdout))
}

/// Run rustor on a file and return errors
fn run_rustor(file: &Path, level: u8) -> Result<Vec<AnalyzerError>, String> {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "rustor-cli",
            "--",
            "analyze",
            file.to_str().unwrap(),
            "--level",
            &level.to_string(),
            "--format",
            "json",
        ])
        .output()
        .map_err(|e| format!("Failed to run rustor: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_rustor_output(&stdout))
}

/// Compare errors from two analyzers
fn compare_errors(phpstan: &[AnalyzerError], rustor: &[AnalyzerError]) -> ComparisonResult {
    let phpstan_set: HashSet<_> = phpstan.iter().map(|e| e.line).collect();
    let rustor_set: HashSet<_> = rustor.iter().map(|e| e.line).collect();

    let missing_in_rustor: Vec<_> = phpstan
        .iter()
        .filter(|e| !rustor_set.contains(&e.line))
        .cloned()
        .collect();

    let extra_in_rustor: Vec<_> = rustor
        .iter()
        .filter(|e| !phpstan_set.contains(&e.line))
        .cloned()
        .collect();

    ComparisonResult {
        phpstan_count: phpstan.len(),
        rustor_count: rustor.len(),
        missing_in_rustor,
        extra_in_rustor,
    }
}

#[derive(Debug)]
struct ComparisonResult {
    phpstan_count: usize,
    rustor_count: usize,
    missing_in_rustor: Vec<AnalyzerError>,
    extra_in_rustor: Vec<AnalyzerError>,
}

impl ComparisonResult {
    fn is_compatible(&self) -> bool {
        self.missing_in_rustor.is_empty() && self.extra_in_rustor.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES_DIR: &str = "tests/integration/phpstan-comparison/fixtures";

    fn test_fixture(name: &str, level: u8) {
        let file = Path::new(FIXTURES_DIR).join(name);

        let phpstan_errors = run_phpstan(&file, level)
            .expect("PHPStan should run successfully");
        let rustor_errors = run_rustor(&file, level)
            .expect("Rustor should run successfully");

        let result = compare_errors(&phpstan_errors, &rustor_errors);

        if !result.is_compatible() {
            eprintln!("\n=== Comparison failed for {} ===", name);
            eprintln!("PHPStan found {} errors, Rustor found {} errors",
                result.phpstan_count, result.rustor_count);

            if !result.missing_in_rustor.is_empty() {
                eprintln!("\nMissing in Rustor (PHPStan found these):");
                for err in &result.missing_in_rustor {
                    eprintln!("  Line {}: [{}] {}", err.line, err.identifier, err.message_contains);
                }
            }

            if !result.extra_in_rustor.is_empty() {
                eprintln!("\nExtra in Rustor (PHPStan didn't find these):");
                for err in &result.extra_in_rustor {
                    eprintln!("  Line {}: [{}] {}", err.line, err.identifier, err.message_contains);
                }
            }

            panic!("PHPStan compatibility check failed");
        }
    }

    #[test]
    #[ignore] // Run with: cargo test --test phpstan_comparison -- --ignored
    fn test_level0_undefined_function() {
        test_fixture("level0_undefined_function.php", 0);
    }

    #[test]
    #[ignore]
    fn test_level0_undefined_class() {
        test_fixture("level0_undefined_class.php", 0);
    }

    #[test]
    #[ignore]
    fn test_level0_undefined_method() {
        test_fixture("level0_undefined_method.php", 0);
    }

    #[test]
    #[ignore]
    fn test_level0_undefined_property() {
        test_fixture("level0_undefined_property.php", 0);
    }

    #[test]
    #[ignore]
    fn test_level0_undefined_constant() {
        test_fixture("level0_undefined_constant.php", 0);
    }

    #[test]
    #[ignore]
    fn test_level0_undefined_static_method() {
        test_fixture("level0_undefined_static_method.php", 0);
    }

    #[test]
    #[ignore]
    fn test_level1_undefined_variable() {
        test_fixture("level1_undefined_variable.php", 1);
    }

    #[test]
    #[ignore]
    fn test_level2_argument_count() {
        test_fixture("level2_argument_count.php", 2);
    }
}
