//! Parser for .php-cs-fixer.php configuration files
//!
//! Extracts configuration using regex patterns to parse PHP code directly.

use std::collections::HashMap;
use std::path::Path;
use regex::Regex;
use thiserror::Error;

use super::whitespace::{IndentStyle, LineEnding, WhitespaceConfig};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid PHP config format: {0}")]
    InvalidFormat(String),
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Configuration for a single fixer rule
#[derive(Debug, Clone, Default)]
pub struct RuleConfig {
    /// Whether the rule is enabled
    pub enabled: bool,
    /// Rule-specific options
    pub options: HashMap<String, ConfigValue>,
}

/// Configuration value types
#[derive(Debug, Clone)]
pub enum ConfigValue {
    Bool(bool),
    String(String),
    Number(i64),
    Array(Vec<String>),
    Map(HashMap<String, String>),
}

/// Finder configuration for file discovery
#[derive(Debug, Clone, Default)]
pub struct FinderConfig {
    /// Paths to include (->in(['src/', 'app/']))
    pub paths: Vec<String>,
    /// Paths to exclude (->exclude(['vendor/', 'var/']))
    pub exclude: Vec<String>,
    /// File name patterns (->name('*.php'))
    pub name_patterns: Vec<String>,
    /// File patterns to ignore (->notName('*.generated.php'))
    pub not_name_patterns: Vec<String>,
}

/// Parsed PHP-CS-Fixer configuration
#[derive(Debug, Clone)]
pub struct PhpCsFixerConfig {
    /// Whitespace configuration
    pub whitespace: WhitespaceConfig,
    /// Enabled rules with their configurations
    pub rules: HashMap<String, RuleConfig>,
    /// Whether risky rules are allowed
    pub risky_allowed: bool,
    /// File finder configuration
    pub finder: FinderConfig,
    /// Cache directory (if configured)
    pub cache_file: Option<String>,
}

impl Default for PhpCsFixerConfig {
    fn default() -> Self {
        Self {
            whitespace: WhitespaceConfig::psr12(),
            rules: HashMap::new(),
            risky_allowed: false,
            finder: FinderConfig::default(),
            cache_file: None,
        }
    }
}

impl PhpCsFixerConfig {
    /// Parse a .php-cs-fixer.php file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        let content = std::fs::read_to_string(path)?;
        parse_php_cs_fixer_config(&content)
    }

    /// Check if a specific rule is enabled
    pub fn is_rule_enabled(&self, name: &str) -> bool {
        self.rules.get(name).map(|r| r.enabled).unwrap_or(false)
    }

    /// Get configuration for a specific rule
    pub fn get_rule_config(&self, name: &str) -> Option<&RuleConfig> {
        self.rules.get(name)
    }
}

/// Parse PHP-CS-Fixer configuration from a string
pub fn parse_php_cs_fixer_config(content: &str) -> Result<PhpCsFixerConfig, ParseError> {
    let mut config = PhpCsFixerConfig::default();

    // Parse indentation: ->setIndent('    ') or ->setIndent("\t")
    config.whitespace.indent = parse_indent(content);

    // Parse line ending: ->setLineEnding("\n") or ->setLineEnding("\r\n")
    config.whitespace.line_ending = parse_line_ending(content);

    // Parse risky allowed: ->setRiskyAllowed(true)
    config.risky_allowed = parse_risky_allowed(content);

    // Parse rules: ->setRules([...])
    config.rules = parse_rules(content)?;

    // Parse finder: PhpCsFixer\Finder::create()->in(...)->exclude(...)
    config.finder = parse_finder(content)?;

    // Parse cache file: ->setCacheFile(...)
    config.cache_file = parse_cache_file(content);

    Ok(config)
}

/// Parse indentation setting
fn parse_indent(content: &str) -> IndentStyle {
    // Match ->setIndent('    ') or ->setIndent("\t")
    let re = Regex::new(r#"->setIndent\s*\(\s*['"](.+?)['"]\s*\)"#).unwrap();

    if let Some(caps) = re.captures(content) {
        let indent_str = &caps[1];
        IndentStyle::from_php_config(indent_str)
    } else {
        IndentStyle::default()
    }
}

/// Parse line ending setting
fn parse_line_ending(content: &str) -> LineEnding {
    // Match ->setLineEnding("\n") or ->setLineEnding("\r\n")
    let re = Regex::new(r#"->setLineEnding\s*\(\s*['"](.+?)['"]\s*\)"#).unwrap();

    if let Some(caps) = re.captures(content) {
        let ending_str = &caps[1];
        LineEnding::from_php_config(ending_str)
    } else {
        LineEnding::default()
    }
}

/// Parse risky allowed setting
fn parse_risky_allowed(content: &str) -> bool {
    // Match ->setRiskyAllowed(true) or ->setRiskyAllowed(false)
    let re = Regex::new(r#"->setRiskyAllowed\s*\(\s*(true|false)\s*\)"#).unwrap();

    if let Some(caps) = re.captures(content) {
        &caps[1] == "true"
    } else {
        false
    }
}

/// Parse rules configuration
fn parse_rules(content: &str) -> Result<HashMap<String, RuleConfig>, ParseError> {
    let mut rules = HashMap::new();

    // Match ->setRules([...]) - capture the array content
    let rules_re = Regex::new(r#"->setRules\s*\(\s*\[([\s\S]*?)\]\s*\)"#)?;

    if let Some(caps) = rules_re.captures(content) {
        let rules_content = &caps[1];

        // Parse preset references like @PSR12, @Symfony
        let preset_re = Regex::new(r#"'@(PSR\d+|PSR-\d+|Symfony|PhpCsFixer)'(?:\s*=>\s*(true|false))?"#)?;
        for cap in preset_re.captures_iter(rules_content) {
            let preset_name = &cap[1];
            let enabled = cap.get(2).map(|m| m.as_str() == "true").unwrap_or(true);

            if enabled {
                // Add all rules from the preset
                let preset_rules = super::presets::get_preset_rules(preset_name);
                for rule_name in preset_rules {
                    rules.insert(rule_name.to_string(), RuleConfig {
                        enabled: true,
                        options: HashMap::new(),
                    });
                }
            }
        }

        // Parse individual rules: 'rule_name' => true/false
        let simple_rule_re = Regex::new(r#"'([a-z_]+)'\s*=>\s*(true|false)"#)?;
        for cap in simple_rule_re.captures_iter(rules_content) {
            let rule_name = cap[1].to_string();
            let enabled = &cap[2] == "true";
            rules.insert(rule_name, RuleConfig {
                enabled,
                options: HashMap::new(),
            });
        }

        // Parse rules with array options: 'rule_name' => ['option' => 'value']
        let array_rule_re = Regex::new(r#"'([a-z_]+)'\s*=>\s*\[([\s\S]*?)\]"#)?;
        for cap in array_rule_re.captures_iter(rules_content) {
            let rule_name = cap[1].to_string();
            let options_str = &cap[2];

            let mut options = HashMap::new();

            // Parse string options: 'option' => 'value'
            let string_opt_re = Regex::new(r#"'([a-z_]+)'\s*=>\s*'([^']+)'"#)?;
            for opt_cap in string_opt_re.captures_iter(options_str) {
                options.insert(
                    opt_cap[1].to_string(),
                    ConfigValue::String(opt_cap[2].to_string()),
                );
            }

            // Parse boolean options: 'option' => true/false
            let bool_opt_re = Regex::new(r#"'([a-z_]+)'\s*=>\s*(true|false)"#)?;
            for opt_cap in bool_opt_re.captures_iter(options_str) {
                options.insert(
                    opt_cap[1].to_string(),
                    ConfigValue::Bool(&opt_cap[2] == "true"),
                );
            }

            // Parse array options: 'option' => ['a', 'b']
            let arr_opt_re = Regex::new(r#"'([a-z_]+)'\s*=>\s*\[([^\]]*)\]"#)?;
            for opt_cap in arr_opt_re.captures_iter(options_str) {
                let values: Vec<String> = Regex::new(r#"'([^']+)'"#)?
                    .captures_iter(&opt_cap[2])
                    .map(|c| c[1].to_string())
                    .collect();
                if !values.is_empty() {
                    options.insert(
                        opt_cap[1].to_string(),
                        ConfigValue::Array(values),
                    );
                }
            }

            rules.insert(rule_name, RuleConfig {
                enabled: true,
                options,
            });
        }
    }

    Ok(rules)
}

/// Parse finder configuration
fn parse_finder(content: &str) -> Result<FinderConfig, ParseError> {
    let mut finder = FinderConfig::default();

    // Match ->in('path') or ->in(['path1', 'path2'])
    let in_re = Regex::new(r#"->in\s*\(\s*(?:'([^']+)'|\[([^\]]+)\]|__DIR__\s*\.\s*'([^']*)')"#)?;
    for cap in in_re.captures_iter(content) {
        if let Some(single) = cap.get(1) {
            finder.paths.push(single.as_str().to_string());
        } else if let Some(array) = cap.get(2) {
            let paths_re = Regex::new(r#"'([^']+)'"#)?;
            for path_cap in paths_re.captures_iter(array.as_str()) {
                finder.paths.push(path_cap[1].to_string());
            }
        } else if let Some(dir_relative) = cap.get(3) {
            finder.paths.push(dir_relative.as_str().trim_start_matches('/').to_string());
        }
    }

    // Match ->exclude('path') or ->exclude(['path1', 'path2'])
    let exclude_re = Regex::new(r#"->exclude\s*\(\s*(?:'([^']+)'|\[([^\]]+)\])"#)?;
    for cap in exclude_re.captures_iter(content) {
        if let Some(single) = cap.get(1) {
            finder.exclude.push(single.as_str().to_string());
        } else if let Some(array) = cap.get(2) {
            let paths_re = Regex::new(r#"'([^']+)'"#)?;
            for path_cap in paths_re.captures_iter(array.as_str()) {
                finder.exclude.push(path_cap[1].to_string());
            }
        }
    }

    // Match ->name('*.php')
    let name_re = Regex::new(r#"->name\s*\(\s*'([^']+)'\s*\)"#)?;
    for cap in name_re.captures_iter(content) {
        finder.name_patterns.push(cap[1].to_string());
    }

    // Match ->notName('*.generated.php')
    let not_name_re = Regex::new(r#"->notName\s*\(\s*'([^']+)'\s*\)"#)?;
    for cap in not_name_re.captures_iter(content) {
        finder.not_name_patterns.push(cap[1].to_string());
    }

    Ok(finder)
}

/// Parse cache file setting
fn parse_cache_file(content: &str) -> Option<String> {
    // Match ->setCacheFile('path') or ->setCacheFile(__DIR__.'/.php-cs-fixer.cache')
    let re = Regex::new(r#"->setCacheFile\s*\(\s*(?:'([^']+)'|__DIR__\s*\.\s*'([^']*)')"#).ok()?;

    re.captures(content).and_then(|cap| {
        cap.get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().trim_start_matches('/').to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_indent_spaces() {
        let content = r#"$config->setIndent('    ')"#;
        assert_eq!(parse_indent(content), IndentStyle::Spaces(4));
    }

    #[test]
    fn test_parse_indent_tabs() {
        let content = r#"$config->setIndent("\t")"#;
        assert_eq!(parse_indent(content), IndentStyle::Tabs);
    }

    #[test]
    fn test_parse_line_ending_lf() {
        let content = r#"$config->setLineEnding("\n")"#;
        assert_eq!(parse_line_ending(content), LineEnding::Lf);
    }

    #[test]
    fn test_parse_line_ending_crlf() {
        let content = r#"$config->setLineEnding("\r\n")"#;
        assert_eq!(parse_line_ending(content), LineEnding::CrLf);
    }

    #[test]
    fn test_parse_risky_allowed() {
        assert!(parse_risky_allowed("->setRiskyAllowed(true)"));
        assert!(!parse_risky_allowed("->setRiskyAllowed(false)"));
        assert!(!parse_risky_allowed("// no risky setting"));
    }

    #[test]
    fn test_parse_simple_rules() {
        let content = r#"
            ->setRules([
                'array_syntax' => true,
                'no_trailing_whitespace' => true,
                'single_quote' => false,
            ])
        "#;
        let rules = parse_rules(content).unwrap();

        assert!(rules.get("array_syntax").unwrap().enabled);
        assert!(rules.get("no_trailing_whitespace").unwrap().enabled);
        assert!(!rules.get("single_quote").unwrap().enabled);
    }

    #[test]
    fn test_parse_rules_with_options() {
        let content = r#"
            ->setRules([
                'ordered_imports' => [
                    'sort_algorithm' => 'alpha',
                    'imports_order' => ['class', 'function', 'const'],
                ],
            ])
        "#;
        let rules = parse_rules(content).unwrap();

        let rule = rules.get("ordered_imports").unwrap();
        assert!(rule.enabled);

        match rule.options.get("sort_algorithm") {
            Some(ConfigValue::String(s)) => assert_eq!(s, "alpha"),
            _ => panic!("Expected string option"),
        }
    }

    #[test]
    fn test_parse_finder_single_path() {
        let content = r#"
            $finder = Finder::create()
                ->in('src')
                ->exclude('vendor')
        "#;
        let finder = parse_finder(content).unwrap();

        assert_eq!(finder.paths, vec!["src"]);
        assert_eq!(finder.exclude, vec!["vendor"]);
    }

    #[test]
    fn test_parse_finder_array_paths() {
        let content = r#"
            $finder = Finder::create()
                ->in(['src', 'app', 'lib'])
                ->exclude(['vendor', 'node_modules'])
        "#;
        let finder = parse_finder(content).unwrap();

        assert_eq!(finder.paths, vec!["src", "app", "lib"]);
        assert_eq!(finder.exclude, vec!["vendor", "node_modules"]);
    }

    #[test]
    fn test_parse_finder_dir_relative() {
        let content = r#"
            $finder = Finder::create()
                ->in(__DIR__.'/src')
        "#;
        let finder = parse_finder(content).unwrap();

        assert_eq!(finder.paths, vec!["src"]);
    }

    #[test]
    fn test_parse_cache_file() {
        assert_eq!(
            parse_cache_file("->setCacheFile('.php-cs-fixer.cache')"),
            Some(".php-cs-fixer.cache".to_string())
        );
        assert_eq!(
            parse_cache_file("->setCacheFile(__DIR__.'/.php-cs-fixer.cache')"),
            Some(".php-cs-fixer.cache".to_string())
        );
    }

    #[test]
    fn test_full_config_parse() {
        let content = r#"
            <?php
            $finder = PhpCsFixer\Finder::create()
                ->in(__DIR__.'/src')
                ->exclude('vendor');

            return (new PhpCsFixer\Config())
                ->setRiskyAllowed(true)
                ->setRules([
                    '@PSR12' => true,
                    'array_syntax' => ['syntax' => 'short'],
                    'no_trailing_whitespace' => true,
                ])
                ->setFinder($finder)
                ->setIndent('    ')
                ->setLineEnding("\n")
                ->setCacheFile(__DIR__.'/.php-cs-fixer.cache');
        "#;

        let config = parse_php_cs_fixer_config(content).unwrap();

        assert!(config.risky_allowed);
        assert_eq!(config.whitespace.indent, IndentStyle::Spaces(4));
        assert_eq!(config.whitespace.line_ending, LineEnding::Lf);
        assert!(config.is_rule_enabled("array_syntax"));
        assert!(config.is_rule_enabled("no_trailing_whitespace"));
        assert_eq!(config.finder.paths, vec!["src"]);
        assert_eq!(config.finder.exclude, vec!["vendor"]);
        assert_eq!(config.cache_file, Some(".php-cs-fixer.cache".to_string()));
    }
}
