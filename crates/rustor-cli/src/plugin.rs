//! Plugin system for custom rules
//!
//! Plugins are external executables that can analyze PHP code and suggest edits.
//!
//! Plugin manifest format (plugin.toml):
//! ```toml
//! name = "my-plugin"
//! version = "1.0.0"
//! description = "My custom rule"
//! command = "./my-plugin"
//! min_php_version = "8.0"
//! category = "custom"
//! ```
//!
//! Plugin protocol:
//! 1. Plugin receives JSON on stdin: { "source": "<?php ...", "file": "path/to/file.php" }
//! 2. Plugin outputs JSON on stdout: { "edits": [...] }
//!
//! Edit format:
//! { "start": 10, "end": 20, "replacement": "new code", "message": "description" }

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Plugin manifest (plugin.toml)
#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub min_php_version: Option<String>,
    #[serde(default)]
    pub category: String,
}

/// Input sent to plugin
#[derive(Debug, Serialize)]
pub struct PluginInput {
    pub source: String,
    pub file: String,
}

/// Output from plugin
#[derive(Debug, Deserialize)]
pub struct PluginOutput {
    #[serde(default)]
    pub edits: Vec<PluginEdit>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Edit suggested by plugin
#[derive(Debug, Clone, Deserialize)]
pub struct PluginEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
    #[serde(default)]
    pub message: String,
}

/// Plugin manager
pub struct PluginManager {
    plugins: HashMap<String, LoadedPlugin>,
    plugin_dir: PathBuf,
}

/// A loaded plugin
#[derive(Debug)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
}

impl PluginManager {
    /// Create a new plugin manager with default plugin directory
    pub fn new() -> Self {
        let plugin_dir = dirs::home_dir()
            .map(|h| h.join(".rustor").join("plugins"))
            .unwrap_or_else(|| PathBuf::from("plugins"));

        Self {
            plugins: HashMap::new(),
            plugin_dir,
        }
    }

    /// Create plugin manager with custom directory
    pub fn with_dir(plugin_dir: PathBuf) -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_dir,
        }
    }

    /// Discover and load plugins from the plugin directory
    pub fn discover_plugins(&mut self) -> Result<usize, String> {
        if !self.plugin_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;

        // Look for plugin.toml files
        for entry in fs::read_dir(&self.plugin_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_dir() {
                let manifest_path = path.join("plugin.toml");
                if manifest_path.exists() {
                    if let Ok(plugin) = self.load_plugin(&manifest_path) {
                        self.plugins.insert(plugin.manifest.name.clone(), plugin);
                        count += 1;
                    }
                }
            } else if path.extension().map(|e| e == "toml").unwrap_or(false) {
                // Single-file plugin manifest
                if let Ok(plugin) = self.load_plugin(&path) {
                    self.plugins.insert(plugin.manifest.name.clone(), plugin);
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Load a plugin from a manifest file
    fn load_plugin(&self, manifest_path: &Path) -> Result<LoadedPlugin, String> {
        let content = fs::read_to_string(manifest_path).map_err(|e| e.to_string())?;
        let manifest: PluginManifest = toml::from_str(&content).map_err(|e| e.to_string())?;

        let plugin_dir = manifest_path.parent().unwrap_or(Path::new("."));

        Ok(LoadedPlugin {
            manifest,
            path: plugin_dir.to_path_buf(),
        })
    }

    /// Get all loaded plugins
    pub fn plugins(&self) -> &HashMap<String, LoadedPlugin> {
        &self.plugins
    }

    /// Get plugin names
    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Run a plugin on source code
    pub fn run_plugin(
        &self,
        name: &str,
        source: &str,
        file_path: &str,
    ) -> Result<Vec<PluginEdit>, String> {
        let plugin = self
            .plugins
            .get(name)
            .ok_or_else(|| format!("Plugin not found: {}", name))?;

        // Resolve command path
        let command_path = if plugin.manifest.command.starts_with("./") {
            plugin.path.join(&plugin.manifest.command[2..])
        } else {
            PathBuf::from(&plugin.manifest.command)
        };

        // Build input
        let input = PluginInput {
            source: source.to_string(),
            file: file_path.to_string(),
        };
        let input_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;

        // Run plugin
        let mut child = Command::new(&command_path)
            .args(&plugin.manifest.args)
            .current_dir(&plugin.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to run plugin {}: {}", name, e))?;

        // Write input
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input_json.as_bytes())
                .map_err(|e| e.to_string())?;
        }

        // Read output
        let output = child.wait_with_output().map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Plugin {} failed: {}", name, stderr));
        }

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let plugin_output: PluginOutput =
            serde_json::from_str(&stdout).map_err(|e| format!("Invalid plugin output: {}", e))?;

        if let Some(error) = plugin_output.error {
            return Err(format!("Plugin error: {}", error));
        }

        Ok(plugin_output.edits)
    }

    /// Run all plugins on source code
    pub fn run_all_plugins(
        &self,
        source: &str,
        file_path: &str,
    ) -> HashMap<String, Result<Vec<PluginEdit>, String>> {
        let mut results = HashMap::new();

        for name in self.plugins.keys() {
            results.insert(name.clone(), self.run_plugin(name, source, file_path));
        }

        results
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_plugin_manifest_parsing() {
        let toml = r#"
            name = "test-plugin"
            version = "1.0.0"
            description = "A test plugin"
            command = "python"
            args = ["plugin.py"]
            min_php_version = "8.0"
            category = "custom"
        "#;

        let manifest: PluginManifest = toml::from_str(toml).unwrap();
        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.command, "python");
        assert_eq!(manifest.args, vec!["plugin.py"]);
    }

    #[test]
    fn test_plugin_discovery() {
        let temp = TempDir::new().unwrap();

        // Create a plugin directory
        let plugin_dir = temp.path().join("my-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        // Write manifest
        let manifest = r#"
            name = "my-plugin"
            version = "1.0.0"
            command = "echo"
        "#;
        fs::write(plugin_dir.join("plugin.toml"), manifest).unwrap();

        // Discover plugins
        let mut manager = PluginManager::with_dir(temp.path().to_path_buf());
        let count = manager.discover_plugins().unwrap();

        assert_eq!(count, 1);
        assert!(manager.plugins.contains_key("my-plugin"));
    }

    #[test]
    fn test_plugin_input_serialization() {
        let input = PluginInput {
            source: "<?php echo 'hello';".to_string(),
            file: "test.php".to_string(),
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("source"));
        assert!(json.contains("file"));
    }

    #[test]
    fn test_plugin_output_parsing() {
        let json = r#"{
            "edits": [
                {
                    "start": 6,
                    "end": 10,
                    "replacement": "print",
                    "message": "Use print instead of echo"
                }
            ]
        }"#;

        let output: PluginOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.edits.len(), 1);
        assert_eq!(output.edits[0].start, 6);
        assert_eq!(output.edits[0].replacement, "print");
    }

    #[test]
    fn test_plugin_output_with_error() {
        let json = r#"{
            "edits": [],
            "error": "Failed to parse PHP"
        }"#;

        let output: PluginOutput = serde_json::from_str(json).unwrap();
        assert!(output.error.is_some());
        assert_eq!(output.error.unwrap(), "Failed to parse PHP");
    }
}
