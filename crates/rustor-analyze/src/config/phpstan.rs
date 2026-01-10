//! PHPStan configuration file parsing

use super::level::Level;
use super::neon::{parse, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse NEON: {0}")]
    ParseError(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Error pattern to ignore
#[derive(Debug, Clone)]
pub struct IgnoreError {
    /// Message pattern (regex or exact match)
    pub message: String,
    /// Optional path pattern
    pub path: Option<String>,
    /// Whether this is a regex pattern
    pub is_regex: bool,
    /// Count limit (None = unlimited)
    pub count: Option<usize>,
    /// Identifier pattern
    pub identifier: Option<String>,
}

/// PHPStan configuration
#[derive(Debug, Clone)]
pub struct PhpStanConfig {
    /// Analysis level (0-9)
    pub level: Level,
    /// Paths to analyze
    pub paths: Vec<PathBuf>,
    /// Paths to exclude
    pub exclude_paths: Vec<PathBuf>,
    /// PHP version (e.g., 80100 for PHP 8.1.0)
    pub php_version: Option<u32>,
    /// Errors to ignore
    pub ignore_errors: Vec<IgnoreError>,
    /// Included config files (already processed)
    pub includes: Vec<PathBuf>,
    /// Treat phpdoc types as certain
    pub treat_phpdoc_types_as_certain: bool,
    /// Check missing type hints
    pub check_missing_typehints: bool,
    /// Report unmatched ignored errors
    pub report_unmatched_ignored_errors: bool,
    /// Parallel processing threads
    pub parallel_max_processes: Option<usize>,
    /// Memory limit
    pub memory_limit: Option<String>,
    /// Custom rule paths
    pub custom_rule_paths: Vec<PathBuf>,
    /// Stub files
    pub stub_files: Vec<PathBuf>,
    /// Bootstrap files
    pub bootstrap_files: Vec<PathBuf>,
    /// PHPStan exact compatibility mode
    pub phpstan_compat: bool,
}

impl Default for PhpStanConfig {
    fn default() -> Self {
        Self {
            level: Level::Level0,
            paths: Vec::new(),
            exclude_paths: Vec::new(),
            php_version: None,
            ignore_errors: Vec::new(),
            includes: Vec::new(),
            treat_phpdoc_types_as_certain: true,
            check_missing_typehints: false,
            report_unmatched_ignored_errors: true,
            parallel_max_processes: None,
            memory_limit: None,
            custom_rule_paths: Vec::new(),
            stub_files: Vec::new(),
            bootstrap_files: Vec::new(),
            phpstan_compat: false,
        }
    }
}

impl PhpStanConfig {
    /// Load configuration from a file
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let mut processed_includes = HashSet::new();
        config.load_file(path, &mut processed_includes)?;
        Ok(config)
    }

    /// Load configuration from a file, tracking processed includes
    fn load_file(&mut self, path: &Path, processed: &mut HashSet<PathBuf>) -> Result<(), ConfigError> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if processed.contains(&canonical) {
            return Ok(()); // Already processed
        }
        processed.insert(canonical.clone());

        let content = fs::read_to_string(path)?;
        let value = parse(&content).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        // Get the base directory for resolving relative paths
        let base_dir = path.parent().unwrap_or(Path::new("."));

        // Process includes first
        if let Some(includes) = value.get("includes") {
            self.process_includes(includes, base_dir, processed)?;
        }

        // Process parameters
        if let Some(params) = value.get("parameters") {
            self.process_parameters(params, base_dir)?;
        }

        // Top-level parameters (for older config format)
        self.process_parameters(&value, base_dir)?;

        Ok(())
    }

    fn process_includes(
        &mut self,
        includes: &Value,
        base_dir: &Path,
        processed: &mut HashSet<PathBuf>,
    ) -> Result<(), ConfigError> {
        if let Some(arr) = includes.as_array() {
            for item in arr {
                if let Some(path_str) = item.as_str() {
                    let include_path = base_dir.join(path_str);
                    if include_path.exists() {
                        self.load_file(&include_path, processed)?;
                        self.includes.push(include_path);
                    }
                }
            }
        }
        Ok(())
    }

    fn process_parameters(&mut self, params: &Value, base_dir: &Path) -> Result<(), ConfigError> {
        let obj = match params.as_object() {
            Some(o) => o,
            None => return Ok(()),
        };

        // Level
        if let Some(level) = obj.get("level") {
            self.level = match level {
                Value::Integer(n) => Level::from_u8(*n as u8),
                Value::String(s) => Level::from_str(s).unwrap_or(Level::Level0),
                _ => Level::Level0,
            };
        }

        // Paths
        if let Some(paths) = obj.get("paths") {
            if let Some(arr) = paths.as_array() {
                for path in arr {
                    if let Some(s) = path.as_str() {
                        self.paths.push(base_dir.join(s));
                    }
                }
            }
        }

        // Exclude paths (multiple possible keys)
        for key in &["excludePaths", "excludes_analyse", "excludes"] {
            if let Some(exclude) = obj.get(*key) {
                self.process_exclude_paths(exclude, base_dir);
            }
        }

        // PHP version
        if let Some(version) = obj.get("phpVersion") {
            if let Some(n) = version.as_i64() {
                self.php_version = Some(n as u32);
            }
        }

        // Ignore errors
        if let Some(ignore) = obj.get("ignoreErrors") {
            self.process_ignore_errors(ignore)?;
        }

        // Boolean flags
        if let Some(Value::Bool(b)) = obj.get("treatPhpDocTypesAsCertain") {
            self.treat_phpdoc_types_as_certain = *b;
        }
        if let Some(Value::Bool(b)) = obj.get("checkMissingTypehints") {
            self.check_missing_typehints = *b;
        }
        if let Some(Value::Bool(b)) = obj.get("reportUnmatchedIgnoredErrors") {
            self.report_unmatched_ignored_errors = *b;
        }

        // Parallel
        if let Some(parallel) = obj.get("parallel") {
            if let Some(parallel_obj) = parallel.as_object() {
                if let Some(Value::Integer(n)) = parallel_obj.get("maximumNumberOfProcesses") {
                    self.parallel_max_processes = Some(*n as usize);
                }
            }
        }

        // Stub files
        if let Some(stubs) = obj.get("stubFiles") {
            if let Some(arr) = stubs.as_array() {
                for stub in arr {
                    if let Some(s) = stub.as_str() {
                        self.stub_files.push(base_dir.join(s));
                    }
                }
            }
        }

        // Bootstrap files
        if let Some(bootstrap) = obj.get("bootstrapFiles") {
            if let Some(arr) = bootstrap.as_array() {
                for file in arr {
                    if let Some(s) = file.as_str() {
                        self.bootstrap_files.push(base_dir.join(s));
                    }
                }
            }
        }

        Ok(())
    }

    fn process_exclude_paths(&mut self, exclude: &Value, base_dir: &Path) {
        match exclude {
            Value::Array(arr) => {
                for path in arr {
                    if let Some(s) = path.as_str() {
                        self.exclude_paths.push(base_dir.join(s));
                    }
                }
            }
            Value::Object(obj) => {
                // Handle { analyse: [...], analyseAndScan: [...] } format
                if let Some(Value::Array(arr)) = obj.get("analyse") {
                    for path in arr {
                        if let Some(s) = path.as_str() {
                            self.exclude_paths.push(base_dir.join(s));
                        }
                    }
                }
                if let Some(Value::Array(arr)) = obj.get("analyseAndScan") {
                    for path in arr {
                        if let Some(s) = path.as_str() {
                            self.exclude_paths.push(base_dir.join(s));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn process_ignore_errors(&mut self, ignore: &Value) -> Result<(), ConfigError> {
        let arr = match ignore.as_array() {
            Some(a) => a,
            None => return Ok(()),
        };

        for item in arr {
            let error = match item {
                Value::String(s) => IgnoreError {
                    message: s.clone(),
                    path: None,
                    is_regex: s.starts_with('#') || s.starts_with('/'),
                    count: None,
                    identifier: None,
                },
                Value::Object(obj) => {
                    let message = obj
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let path = obj.get("path").and_then(|v| v.as_str()).map(String::from);
                    let count = obj.get("count").and_then(|v| v.as_i64()).map(|n| n as usize);
                    let identifier = obj
                        .get("identifier")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    IgnoreError {
                        is_regex: message.starts_with('#') || message.starts_with('/'),
                        message,
                        path,
                        count,
                        identifier,
                    }
                }
                _ => continue,
            };
            self.ignore_errors.push(error);
        }

        Ok(())
    }

    /// Find phpstan.neon or phpstan.neon.dist in the current directory
    pub fn find_config(dir: &Path) -> Option<PathBuf> {
        let candidates = ["phpstan.neon", "phpstan.neon.dist"];
        for name in &candidates {
            let path = dir.join(name);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    /// Check if a path should be excluded
    pub fn is_excluded(&self, path: &Path) -> bool {
        for exclude in &self.exclude_paths {
            if path.starts_with(exclude) {
                return true;
            }
            // Check glob patterns
            let exclude_str = exclude.to_string_lossy();
            if exclude_str.contains('*') {
                // Simple glob matching
                if let Ok(pattern) = glob::Pattern::new(&exclude_str) {
                    if pattern.matches_path(path) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if an error should be ignored
    pub fn should_ignore_error(&self, message: &str, path: &Path, identifier: Option<&str>) -> bool {
        for ignore in &self.ignore_errors {
            // Check identifier first if specified
            if let Some(ignore_id) = &ignore.identifier {
                if let Some(error_id) = identifier {
                    if ignore_id == error_id {
                        return true;
                    }
                }
            }

            // Check path pattern
            if let Some(path_pattern) = &ignore.path {
                let path_str = path.to_string_lossy();
                if !path_str.contains(path_pattern) {
                    continue;
                }
            }

            // Check message pattern
            if ignore.is_regex {
                // Simple regex check (strip leading/trailing delimiters)
                let pattern = ignore
                    .message
                    .trim_start_matches(|c| c == '#' || c == '/')
                    .trim_end_matches(|c| c == '#' || c == '/');
                if let Ok(re) = fnmatch_regex::glob_to_regex(pattern) {
                    if re.is_match(message) {
                        return true;
                    }
                }
            } else if message.contains(&ignore.message) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_simple_config() {
        let content = r#"
parameters:
    level: 5
    paths:
        - src/
        - tests/
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let config = PhpStanConfig::load(file.path()).unwrap();
        assert_eq!(config.level, Level::Level5);
        assert_eq!(config.paths.len(), 2);
    }

    #[test]
    fn test_parse_ignore_errors() {
        let content = r#"
parameters:
    ignoreErrors:
        - '#Call to undefined function#'
        -
            message: '#Variable \$foo#'
            path: src/Legacy.php
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let config = PhpStanConfig::load(file.path()).unwrap();
        assert_eq!(config.ignore_errors.len(), 2);
        assert!(config.ignore_errors[0].is_regex);
    }

    #[test]
    fn test_should_ignore_error() {
        let mut config = PhpStanConfig::default();
        config.ignore_errors.push(IgnoreError {
            message: "undefined function".to_string(),
            path: None,
            is_regex: false,
            count: None,
            identifier: None,
        });

        assert!(config.should_ignore_error(
            "Call to undefined function foo()",
            Path::new("test.php"),
            None
        ));
        assert!(!config.should_ignore_error(
            "Undefined variable $bar",
            Path::new("test.php"),
            None
        ));
    }

    #[test]
    fn test_includes_merge_parameters() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create baseline file
        let baseline_content = r#"
parameters:
    ignoreErrors:
        -
            message: '#Error from baseline#'
            identifier: baseline.error
"#;
        let baseline_path = temp_dir.path().join("baseline.neon");
        std::fs::write(&baseline_path, baseline_content).unwrap();

        // Create main config that includes baseline
        let main_content = r#"
includes:
    - baseline.neon
parameters:
    level: 5
    ignoreErrors:
        - '#Error from main#'
"#;
        let main_path = temp_dir.path().join("phpstan.neon");
        std::fs::write(&main_path, main_content).unwrap();

        // Load the config
        let config = PhpStanConfig::load(&main_path).unwrap();

        // Check that level is set from main config
        assert_eq!(config.level, Level::Level5);

        // Check that ignore errors from BOTH files are present
        assert!(
            config.ignore_errors.len() >= 2,
            "Expected at least 2 ignore errors, got {}: {:?}",
            config.ignore_errors.len(),
            config.ignore_errors
        );

        // Check that baseline error is present
        let has_baseline_error = config
            .ignore_errors
            .iter()
            .any(|e| e.message.contains("Error from baseline"));
        assert!(has_baseline_error, "Missing error from baseline file");

        // Check that main config error is present
        let has_main_error = config
            .ignore_errors
            .iter()
            .any(|e| e.message.contains("Error from main"));
        assert!(has_main_error, "Missing error from main config file");

        // Check that includes were tracked
        assert_eq!(config.includes.len(), 1);
    }

    #[test]
    fn test_includes_with_tab_indentation() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create baseline file with tab indentation (like PHPStan generates)
        let baseline_content = "parameters:\n\tignoreErrors:\n\t\t-\n\t\t\tmessage: '#Tab indented error#'\n\t\t\tidentifier: tab.error\n";
        let baseline_path = temp_dir.path().join("baseline.neon");
        std::fs::write(&baseline_path, baseline_content).unwrap();

        // Create main config
        let main_content = "includes:\n    - baseline.neon\nparameters:\n    level: 3\n";
        let main_path = temp_dir.path().join("phpstan.neon");
        std::fs::write(&main_path, main_content).unwrap();

        // Load the config
        let config = PhpStanConfig::load(&main_path).unwrap();

        // Check that the tab-indented error was parsed
        let has_tab_error = config
            .ignore_errors
            .iter()
            .any(|e| e.message.contains("Tab indented error"));
        assert!(
            has_tab_error,
            "Failed to parse tab-indented baseline. Errors: {:?}",
            config.ignore_errors
        );
    }

    #[test]
    fn test_example_phpstan_config() {
        // Test loading the actual example files if they exist
        // Use workspace root relative path
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let example_path = workspace_root.join("examples/phpstan/phpstan.neon.dist");

        if !example_path.exists() {
            eprintln!("Skipping test - example not found at: {:?}", example_path);
            return;
        }

        let config = PhpStanConfig::load(&example_path).unwrap();

        // Should have level 6
        assert_eq!(config.level, Level::Level6);

        // Should have many paths configured
        assert!(!config.paths.is_empty(), "Expected paths to be configured");

        // Should have ignore errors from baseline
        assert!(
            !config.ignore_errors.is_empty(),
            "Expected ignore errors from baseline file"
        );

        // Should have the baseline included
        assert!(
            !config.includes.is_empty(),
            "Expected baseline to be included"
        );

        // Print some stats for debugging
        eprintln!("Loaded config:");
        eprintln!("  Level: {:?}", config.level);
        eprintln!("  Paths: {}", config.paths.len());
        eprintln!("  Exclude paths: {}", config.exclude_paths.len());
        eprintln!("  Ignore errors: {}", config.ignore_errors.len());
        eprintln!("  Includes: {:?}", config.includes);
    }
}
