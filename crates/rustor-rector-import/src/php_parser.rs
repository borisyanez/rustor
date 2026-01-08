//! PHP file parser using regex patterns
//!
//! This module extracts information from Rector PHP rule files without
//! a full PHP parser. It uses regex patterns to find common structures.

use regex::Regex;
use std::path::Path;

/// Parsed content from a Rector rule file
#[derive(Debug, Default)]
pub struct ParsedRuleFile {
    /// Class name (e.g., "IsNullRector")
    pub class_name: Option<String>,

    /// Namespace (e.g., "Rector\\CodeQuality\\Rector\\Identical")
    pub namespace: Option<String>,

    /// Node types from getNodeTypes() (e.g., ["FuncCall", "Identical"])
    pub node_types: Vec<String>,

    /// Description from getRuleDefinition()
    pub description: Option<String>,

    /// Before code sample
    pub before_code: Option<String>,

    /// After code sample
    pub after_code: Option<String>,

    /// Body of refactor() method
    pub refactor_body: Option<String>,

    /// Whether it implements ConfigurableRectorInterface
    pub is_configurable: bool,

    /// Raw content for further analysis
    pub raw_content: String,
}

/// Parse a Rector PHP rule file
pub fn parse_rule_file(content: &str, _path: &Path) -> ParsedRuleFile {
    let mut result = ParsedRuleFile {
        raw_content: content.to_string(),
        ..Default::default()
    };

    // Extract namespace
    result.namespace = extract_namespace(content);

    // Extract class name
    result.class_name = extract_class_name(content);

    // Check if configurable
    result.is_configurable = content.contains("ConfigurableRectorInterface");

    // Extract node types
    result.node_types = extract_node_types(content);

    // Extract description
    result.description = extract_description(content);

    // Extract code samples
    if let Some((before, after)) = extract_code_samples(content) {
        result.before_code = Some(before);
        result.after_code = Some(after);
    }

    // Extract refactor body
    result.refactor_body = extract_refactor_body(content);

    result
}

/// Extract namespace from PHP file
fn extract_namespace(content: &str) -> Option<String> {
    let re = Regex::new(r"namespace\s+([^;]+);").ok()?;
    re.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
}

/// Extract class name from PHP file
fn extract_class_name(content: &str) -> Option<String> {
    let re = Regex::new(r"(?:final\s+)?class\s+(\w+Rector)").ok()?;
    re.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extract node types from getNodeTypes()
fn extract_node_types(content: &str) -> Vec<String> {
    // Match: return [FuncCall::class, Identical::class];
    let re = Regex::new(r"public\s+function\s+getNodeTypes\s*\(\s*\)\s*:\s*array\s*\{[^}]*return\s*\[([\s\S]*?)\];").ok();

    if let Some(re) = re {
        if let Some(caps) = re.captures(content) {
            if let Some(array_content) = caps.get(1) {
                let node_re = Regex::new(r"(\w+)::class").ok();
                if let Some(node_re) = node_re {
                    return node_re
                        .captures_iter(array_content.as_str())
                        .filter_map(|c| c.get(1))
                        .map(|m| m.as_str().to_string())
                        .collect();
                }
            }
        }
    }

    Vec::new()
}

/// Extract description from getRuleDefinition()
fn extract_description(content: &str) -> Option<String> {
    // Match: new RuleDefinition('Description here', [
    let re = Regex::new(r#"new\s+RuleDefinition\s*\(\s*['"]([\s\S]*?)['"]"#).ok()?;
    re.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
}

/// Extract before/after code samples
fn extract_code_samples(content: &str) -> Option<(String, String)> {
    // Match CodeSample with heredoc syntax
    // Pattern: new CodeSample(<<<'CODE_SAMPLE' ... CODE_SAMPLE, <<<'CODE_SAMPLE' ... CODE_SAMPLE)
    let re = Regex::new(
        r#"new\s+(?:Configured)?CodeSample\s*\(\s*<<<\s*['"]?CODE_SAMPLE['"]?\s*([\s\S]*?)CODE_SAMPLE\s*,\s*<<<\s*['"]?CODE_SAMPLE['"]?\s*([\s\S]*?)CODE_SAMPLE"#
    ).ok()?;

    if let Some(caps) = re.captures(content) {
        let before = caps.get(1).map(|m| m.as_str().trim().to_string())?;
        let after = caps.get(2).map(|m| m.as_str().trim().to_string())?;
        return Some((before, after));
    }

    // Try alternative pattern with regular strings
    let re2 = Regex::new(
        r#"new\s+CodeSample\s*\(\s*[']([\s\S]*?)[']\s*,\s*[']([\s\S]*?)[']"#
    ).ok()?;

    if let Some(caps) = re2.captures(content) {
        let before = caps.get(1).map(|m| m.as_str().trim().to_string())?;
        let after = caps.get(2).map(|m| m.as_str().trim().to_string())?;
        return Some((before, after));
    }

    None
}

/// Extract the body of refactor() method
fn extract_refactor_body(content: &str) -> Option<String> {
    // This is tricky because we need to match balanced braces
    // For now, use a simplified approach that works for most cases

    let refactor_start = content.find("public function refactor(")?;
    let body_start = content[refactor_start..].find('{')?;
    let absolute_start = refactor_start + body_start;

    // Count braces to find matching close
    let mut depth = 0;
    let mut end_pos = None;

    for (i, c) in content[absolute_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end_pos = Some(absolute_start + i);
                    break;
                }
            }
            _ => {}
        }
    }

    end_pos.map(|end| content[absolute_start + 1..end].trim().to_string())
}

/// Determine category from namespace or file path
pub fn extract_category(namespace: &Option<String>, path: &Path) -> String {
    // Try to get from namespace first
    if let Some(ns) = namespace {
        // e.g., "Rector\CodeQuality\Rector\Identical" -> "CodeQuality"
        let parts: Vec<&str> = ns.split('\\').collect();
        if parts.len() >= 2 && parts[0] == "Rector" {
            return parts[1].to_string();
        }
    }

    // Fall back to path
    // e.g., "rules/CodeQuality/Rector/Identical/IsNullRector.php" -> "CodeQuality"
    let path_str = path.to_string_lossy();
    if let Some(rules_idx) = path_str.find("rules/") {
        let after_rules = &path_str[rules_idx + 6..];
        if let Some(slash_idx) = after_rules.find('/') {
            return after_rules[..slash_idx].to_string();
        }
    }

    "Unknown".to_string()
}

/// Extract minimum PHP version from category name
pub fn extract_php_version(category: &str) -> Option<String> {
    // Categories like "Php80", "Php81" indicate minimum version
    let re = Regex::new(r"^Php(\d)(\d)$").ok()?;
    re.captures(category).map(|caps| {
        let major = caps.get(1).unwrap().as_str();
        let minor = caps.get(2).unwrap().as_str();
        format!("{}.{}", major, minor)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_namespace() {
        let content = "<?php\n\nnamespace Rector\\CodeQuality\\Rector\\Identical;\n";
        assert_eq!(
            extract_namespace(content),
            Some("Rector\\CodeQuality\\Rector\\Identical".to_string())
        );
    }

    #[test]
    fn test_extract_class_name() {
        let content = "final class IsNullRector extends AbstractRector";
        assert_eq!(extract_class_name(content), Some("IsNullRector".to_string()));
    }

    #[test]
    fn test_extract_node_types() {
        let content = r#"
            public function getNodeTypes(): array
            {
                return [FuncCall::class, Identical::class];
            }
        "#;
        let types = extract_node_types(content);
        assert_eq!(types, vec!["FuncCall", "Identical"]);
    }

    #[test]
    fn test_extract_description() {
        let content = r#"
            return new RuleDefinition('Replace is_null() with === null', [
        "#;
        assert_eq!(
            extract_description(content),
            Some("Replace is_null() with === null".to_string())
        );
    }

    #[test]
    fn test_extract_category() {
        let ns = Some("Rector\\CodeQuality\\Rector\\Identical".to_string());
        let path = PathBuf::from("rules/CodeQuality/Rector/Identical/IsNullRector.php");
        assert_eq!(extract_category(&ns, &path), "CodeQuality");
    }

    #[test]
    fn test_extract_php_version() {
        assert_eq!(extract_php_version("Php80"), Some("8.0".to_string()));
        assert_eq!(extract_php_version("Php74"), Some("7.4".to_string()));
        assert_eq!(extract_php_version("CodeQuality"), None);
    }
}
