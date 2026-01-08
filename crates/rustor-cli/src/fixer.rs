//! PHP-CS-Fixer integration for rustor CLI
//!
//! Provides formatting fixers compatible with PHP-CS-Fixer configuration.

use std::path::Path;
use anyhow::Result;
use colored::*;
use rustor_core::apply_edits;
use rustor_fixer::{
    PhpCsFixerConfig, FixerRegistry, FixerConfig,
    config::{IndentStyle, LineEnding},
};

/// Load fixer configuration from a .php-cs-fixer.php file
pub fn load_fixer_config(path: &Path) -> Result<PhpCsFixerConfig> {
    PhpCsFixerConfig::from_file(path)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))
}

/// Run fixers on a single file
pub fn run_fixers_on_file(
    source: &str,
    registry: &FixerRegistry,
    config: &FixerConfig,
    fixer_names: Option<&[&str]>,
) -> Result<(String, Vec<rustor_core::Edit>)> {
    let (fixed, edits) = if let Some(names) = fixer_names {
        registry.check(source, names, config)
    } else {
        registry.check_all(source, config)
    };

    if edits.is_empty() {
        return Ok((source.to_string(), vec![]));
    }

    Ok((fixed, edits))
}

/// Run fixers with a preset
pub fn run_fixers_with_preset(
    source: &str,
    registry: &FixerRegistry,
    config: &FixerConfig,
    preset: &str,
) -> Result<(String, Vec<rustor_core::Edit>)> {
    let (fixed, edits) = registry.check_preset(source, preset, config);

    if edits.is_empty() {
        return Ok((source.to_string(), vec![]));
    }

    Ok((fixed, edits))
}

/// List available fixers
pub fn list_fixers(registry: &FixerRegistry) {
    println!("{}", "Available fixers:".bold());
    println!();

    let mut fixers = registry.list();
    fixers.sort_by(|a, b| a.php_cs_fixer_name.cmp(&b.php_cs_fixer_name));

    for info in fixers {
        let risky_marker = if info.is_risky {
            " [risky]".yellow().to_string()
        } else {
            String::new()
        };

        println!(
            "  {} - {} (priority: {}){}",
            info.php_cs_fixer_name.green(),
            info.description,
            info.priority,
            risky_marker
        );
    }

    println!();
    println!("{}", "Presets:".bold());
    println!("  {} - PSR-12 coding standard (~50 fixers)", "psr12".green());
    println!("  {} - Symfony coding standard", "symfony".green());
    println!("  {} - PHP-CS-Fixer coding standard", "phpcsfixer".green());
}

/// Create FixerConfig from PhpCsFixerConfig
pub fn config_from_php_cs_fixer(php_config: &PhpCsFixerConfig) -> FixerConfig {
    use rustor_fixer::{ConfigValue, PhpConfigValue};
    use std::collections::HashMap;

    // Merge all rule options into a single HashMap
    let mut options: HashMap<String, ConfigValue> = HashMap::new();

    for (_rule_name, rule_config) in &php_config.rules {
        for (opt_name, opt_value) in &rule_config.options {
            // Convert from php_parser::ConfigValue to fixer::ConfigValue
            let converted = match opt_value {
                PhpConfigValue::Bool(b) => ConfigValue::Bool(*b),
                PhpConfigValue::String(s) => ConfigValue::String(s.clone()),
                PhpConfigValue::Number(n) => ConfigValue::Number(*n),
                PhpConfigValue::Array(arr) => ConfigValue::Array(arr.clone()),
                PhpConfigValue::Map(map) => {
                    // Convert Map to StringMap
                    ConfigValue::StringMap(map.clone())
                }
            };
            options.insert(opt_name.clone(), converted);
        }
    }

    FixerConfig {
        indent: php_config.whitespace.indent,
        line_ending: php_config.whitespace.line_ending,
        options,
    }
}

/// Create a default FixerConfig (PSR-12 style)
pub fn default_fixer_config() -> FixerConfig {
    FixerConfig {
        indent: IndentStyle::Spaces(4),
        line_ending: LineEnding::Lf,
        options: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_fixers_on_file() {
        let registry = FixerRegistry::new();
        let config = default_fixer_config();

        let source = "<?php\n$a = TRUE;   \n";
        let (fixed, edits) = run_fixers_on_file(source, &registry, &config, None).unwrap();

        // Should have edits for TRUE -> true and trailing whitespace
        assert!(!edits.is_empty());
        assert!(fixed.contains("true"));
        assert!(!fixed.contains("   \n")); // No trailing whitespace
    }

    #[test]
    fn test_run_fixers_with_preset() {
        let registry = FixerRegistry::new();
        let config = default_fixer_config();

        let source = "<?php\nIF ($a) { }\n";
        let (fixed, edits) = run_fixers_with_preset(source, &registry, &config, "psr12").unwrap();

        // PSR-12 requires lowercase keywords
        assert!(!edits.is_empty());
        assert!(fixed.contains("if"));
    }
}
