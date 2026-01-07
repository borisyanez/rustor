//! Configuration file support for rustor
//!
//! Loads `.rustor.toml` from current directory or parent directories.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Configuration file structure
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub rules: RulesConfig,
    pub paths: PathsConfig,
    pub output: OutputConfig,
    pub php: PhpConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PhpConfig {
    /// Target PHP version (e.g., "7.4", "8.0")
    /// Only rules compatible with this version will run
    pub version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RulesConfig {
    /// Preset to use (recommended, performance, modernize, all)
    pub preset: Option<String>,
    /// If set, only these rules will run (overrides preset)
    pub enabled: Option<Vec<String>>,
    /// Rules to exclude (applied after enabled/preset)
    pub disabled: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    /// Glob patterns to exclude from processing
    pub exclude: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Output format: "text" or "json"
    pub format: Option<String>,
}

impl Config {
    /// Load config from `.rustor.toml` searching from current directory upward
    pub fn load() -> Result<Option<(Config, PathBuf)>> {
        Self::load_from(std::env::current_dir()?)
    }

    /// Load config searching from the given directory upward
    pub fn load_from(start_dir: PathBuf) -> Result<Option<(Config, PathBuf)>> {
        let mut current = Some(start_dir.as_path());

        while let Some(dir) = current {
            let config_path = dir.join(".rustor.toml");
            if config_path.exists() {
                let contents = std::fs::read_to_string(&config_path)
                    .with_context(|| format!("Failed to read {}", config_path.display()))?;
                let config: Config = toml::from_str(&contents)
                    .with_context(|| format!("Failed to parse {}", config_path.display()))?;
                return Ok(Some((config, config_path)));
            }
            current = dir.parent();
        }

        Ok(None)
    }

    /// Load config from a specific path
    pub fn load_path(path: &Path) -> Result<Config> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(config)
    }

    /// Compute the effective set of enabled rules
    pub fn effective_rules(&self, all_rules: &[&str], cli_rules: &[String]) -> HashSet<String> {
        // CLI rules override config completely
        if !cli_rules.is_empty() {
            return cli_rules.iter().cloned().collect();
        }

        // Start with enabled rules from config, or all rules if not specified
        let mut rules: HashSet<String> = match &self.rules.enabled {
            Some(enabled) => enabled.iter().cloned().collect(),
            None => all_rules.iter().map(|s| s.to_string()).collect(),
        };

        // Remove disabled rules
        for disabled in &self.rules.disabled {
            rules.remove(disabled);
        }

        rules
    }

    /// Check if a path should be excluded based on config patterns
    pub fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.paths.exclude {
            // Try glob matching
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches(&path_str) {
                    return true;
                }
                // Also try matching against just the file/dir name
                if let Some(file_name) = path.file_name() {
                    if glob_pattern.matches(&file_name.to_string_lossy()) {
                        return true;
                    }
                }
            }

            // Also do simple prefix/contains matching for directory patterns
            if pattern.ends_with('/') {
                let dir_pattern = pattern.trim_end_matches('/');
                if path_str.contains(&format!("/{}/", dir_pattern))
                    || path_str.starts_with(&format!("{}/", dir_pattern))
                {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_config(dir: &Path, content: &str) {
        fs::write(dir.join(".rustor.toml"), content).unwrap();
    }

    #[test]
    fn test_load_basic_config() {
        let temp = TempDir::new().unwrap();
        create_config(
            temp.path(),
            r#"
[rules]
enabled = ["array_push", "sizeof"]
disabled = ["sizeof"]

[paths]
exclude = ["vendor/", "*.generated.php"]

[output]
format = "json"
"#,
        );

        let (config, path) = Config::load_from(temp.path().to_path_buf())
            .unwrap()
            .unwrap();

        assert_eq!(path, temp.path().join(".rustor.toml"));
        assert_eq!(
            config.rules.enabled,
            Some(vec!["array_push".to_string(), "sizeof".to_string()])
        );
        assert_eq!(config.rules.disabled, vec!["sizeof".to_string()]);
        assert_eq!(
            config.paths.exclude,
            vec!["vendor/".to_string(), "*.generated.php".to_string()]
        );
        assert_eq!(config.output.format, Some("json".to_string()));
    }

    #[test]
    fn test_load_empty_config() {
        let temp = TempDir::new().unwrap();
        create_config(temp.path(), "");

        let (config, _) = Config::load_from(temp.path().to_path_buf())
            .unwrap()
            .unwrap();

        assert!(config.rules.enabled.is_none());
        assert!(config.rules.disabled.is_empty());
        assert!(config.paths.exclude.is_empty());
        assert!(config.output.format.is_none());
    }

    #[test]
    fn test_no_config_found() {
        let temp = TempDir::new().unwrap();
        let result = Config::load_from(temp.path().to_path_buf()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_effective_rules_cli_override() {
        let config = Config::default();
        let all_rules = &["array_push", "sizeof", "is_null"];
        let cli_rules = vec!["array_push".to_string()];

        let effective = config.effective_rules(all_rules, &cli_rules);

        assert_eq!(effective.len(), 1);
        assert!(effective.contains("array_push"));
    }

    #[test]
    fn test_effective_rules_config_enabled() {
        let config = Config {
            rules: RulesConfig {
                preset: None,
                enabled: Some(vec!["array_push".to_string(), "sizeof".to_string()]),
                disabled: vec![],
            },
            ..Default::default()
        };
        let all_rules = &["array_push", "sizeof", "is_null"];

        let effective = config.effective_rules(all_rules, &[]);

        assert_eq!(effective.len(), 2);
        assert!(effective.contains("array_push"));
        assert!(effective.contains("sizeof"));
    }

    #[test]
    fn test_effective_rules_with_disabled() {
        let config = Config {
            rules: RulesConfig {
                preset: None,
                enabled: None,
                disabled: vec!["sizeof".to_string()],
            },
            ..Default::default()
        };
        let all_rules = &["array_push", "sizeof", "is_null"];

        let effective = config.effective_rules(all_rules, &[]);

        assert_eq!(effective.len(), 2);
        assert!(effective.contains("array_push"));
        assert!(effective.contains("is_null"));
        assert!(!effective.contains("sizeof"));
    }

    #[test]
    fn test_should_exclude_glob() {
        let config = Config {
            paths: PathsConfig {
                exclude: vec!["*.generated.php".to_string()],
            },
            ..Default::default()
        };

        assert!(config.should_exclude(Path::new("foo.generated.php")));
        assert!(!config.should_exclude(Path::new("foo.php")));
    }

    #[test]
    fn test_should_exclude_directory() {
        let config = Config {
            paths: PathsConfig {
                exclude: vec!["vendor/".to_string()],
            },
            ..Default::default()
        };

        assert!(config.should_exclude(Path::new("project/vendor/autoload.php")));
        assert!(config.should_exclude(Path::new("vendor/package/file.php")));
        assert!(!config.should_exclude(Path::new("src/vendor.php")));
    }
}
