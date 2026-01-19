//! YAML rule loader
//!
//! Load YAML rules from files, directories, or strings.

use std::fs;
use std::path::Path;
use thiserror::Error;

use super::interpreter::YamlRuleInterpreter;
use super::schema::YamlRule;

/// Errors that can occur when loading YAML rules
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Invalid rule: {0}")]
    Validation(String),
}

/// Load a single YAML rule from a string
pub fn load_rules_from_string(yaml: &str) -> Result<Vec<YamlRuleInterpreter>, LoadError> {
    // Try to parse as a single rule first
    if let Ok(rule) = serde_yaml::from_str::<YamlRule>(yaml) {
        rule.validate().map_err(LoadError::Validation)?;
        return Ok(vec![YamlRuleInterpreter::new(rule)]);
    }

    // Try to parse as a list of rules
    let rules: Vec<YamlRule> = serde_yaml::from_str(yaml)?;
    let mut interpreters = Vec::with_capacity(rules.len());

    for rule in rules {
        rule.validate().map_err(LoadError::Validation)?;
        interpreters.push(YamlRuleInterpreter::new(rule));
    }

    Ok(interpreters)
}

/// Load YAML rules from a file
pub fn load_rules_from_file(path: &Path) -> Result<Vec<YamlRuleInterpreter>, LoadError> {
    let content = fs::read_to_string(path)?;
    load_rules_from_string(&content)
}

/// Load all YAML rules from a directory
pub fn load_rules_from_dir(dir: &Path) -> Result<Vec<YamlRuleInterpreter>, LoadError> {
    let mut all_rules = Vec::new();

    if !dir.is_dir() {
        return Err(LoadError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Directory not found: {}", dir.display()),
        )));
    }

    // Walk the directory recursively
    walk_dir(dir, &mut all_rules)?;

    Ok(all_rules)
}

fn walk_dir(dir: &Path, rules: &mut Vec<YamlRuleInterpreter>) -> Result<(), LoadError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            walk_dir(&path, rules)?;
        } else if let Some(ext) = path.extension() {
            if ext == "yaml" || ext == "yml" {
                match load_rules_from_file(&path) {
                    Ok(loaded) => rules.extend(loaded),
                    Err(e) => {
                        eprintln!("Warning: Failed to load {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Validate a YAML rule without loading it
pub fn validate_rule_string(yaml: &str) -> Result<(), LoadError> {
    let rule: YamlRule = serde_yaml::from_str(yaml)?;
    rule.validate().map_err(LoadError::Validation)?;
    Ok(())
}

/// Information about a loaded rule
#[derive(Debug, Clone)]
pub struct RuleInfo {
    pub name: String,
    pub description: String,
    pub category: String,
    pub min_php: Option<String>,
    pub test_count: usize,
}

impl From<&YamlRule> for RuleInfo {
    fn from(rule: &YamlRule) -> Self {
        Self {
            name: rule.name.clone(),
            description: rule.description.clone(),
            category: rule.category.clone(),
            min_php: rule.min_php.clone(),
            test_count: rule.tests.len(),
        }
    }
}

/// Get information about rules in a file without fully loading them
pub fn get_rule_info(path: &Path) -> Result<Vec<RuleInfo>, LoadError> {
    let content = fs::read_to_string(path)?;

    // Try single rule
    if let Ok(rule) = serde_yaml::from_str::<YamlRule>(&content) {
        return Ok(vec![RuleInfo::from(&rule)]);
    }

    // Try list of rules
    let rules: Vec<YamlRule> = serde_yaml::from_str(&content)?;
    Ok(rules.iter().map(RuleInfo::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_single_rule() {
        let yaml = r#"
name: test_rule
description: A test rule
category: code_quality

match:
  node: FuncCall
  name: old_func
  args:
    - capture: $arg

replace: "new_func($arg)"

tests:
  - input: "old_func($x)"
    output: "new_func($x)"
"#;

        let rules = load_rules_from_string(yaml).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].rule().name, "test_rule");
    }

    #[test]
    fn test_load_multiple_rules() {
        let yaml = r#"
- name: rule_one
  description: First rule
  match:
    node: FuncCall
    name: func_one
    args: []
  replace: "replacement_one()"
  tests: []

- name: rule_two
  description: Second rule
  match:
    node: FuncCall
    name: func_two
    args: []
  replace: "replacement_two()"
  tests: []
"#;

        let rules = load_rules_from_string(yaml).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].rule().name, "rule_one");
        assert_eq!(rules[1].rule().name, "rule_two");
    }

    #[test]
    fn test_validation_error() {
        let yaml = r#"
name: ""
description: Missing name
match:
  node: FuncCall
replace: "x"
"#;

        let result = load_rules_from_string(yaml);
        assert!(result.is_err());
    }
}
