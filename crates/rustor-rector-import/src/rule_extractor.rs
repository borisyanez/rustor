//! Rule extractor - builds RectorRule from parsed PHP files

use crate::pattern_detector::{detect_pattern_from_samples, detect_pattern_with_ast};
use crate::php_parser::{extract_category, extract_php_version, parse_rule_file};
use crate::{ImportResult, RectorRule, RulePattern};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Extract all rules from a Rector repository
pub fn extract_rules_from_repo(repo_path: &Path) -> ImportResult {
    let mut result = ImportResult::new();

    let rules_path = repo_path.join("rules");
    if !rules_path.exists() {
        result.warnings.push(format!(
            "Rules directory not found: {}",
            rules_path.display()
        ));
        return result;
    }

    // Walk through all PHP files in rules/
    for entry in WalkDir::new(&rules_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map(|ext| ext == "php").unwrap_or(false)
                && e.path()
                    .file_name()
                    .map(|n| n.to_string_lossy().ends_with("Rector.php"))
                    .unwrap_or(false)
        })
    {
        let path = entry.path();

        match extract_rule_from_file(path) {
            Ok(Some(rule)) => result.rules.push(rule),
            Ok(None) => {
                result.warnings.push(format!(
                    "Could not extract rule from: {}",
                    path.display()
                ));
            }
            Err(e) => {
                result.failed.push((path.to_string_lossy().to_string(), e));
            }
        }
    }

    result
}

/// Extract a single rule from a PHP file
pub fn extract_rule_from_file(path: &Path) -> Result<Option<RectorRule>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;

    let parsed = parse_rule_file(&content, path);

    // Must have a class name
    let name = match parsed.class_name {
        Some(n) => n,
        None => return Ok(None),
    };

    // Determine category
    let category = extract_category(&parsed.namespace, path);

    // Determine PHP version from category if applicable
    let min_php_version = extract_php_version(&category);

    // Detect pattern from refactor body using AST analysis + regex fallback
    let mut pattern = if let Some(ref body) = parsed.refactor_body {
        detect_pattern_with_ast(body, &parsed.node_types)
    } else {
        RulePattern::Unknown
    };

    // If pattern detection from refactor body failed, try code samples
    if matches!(pattern, RulePattern::Unknown | RulePattern::Complex { .. }) {
        if let (Some(ref before), Some(ref after)) = (&parsed.before_code, &parsed.after_code) {
            let sample_pattern = detect_pattern_from_samples(before, after, &parsed.node_types);
            // Only use sample-based detection if it gives us a specific pattern
            if !matches!(sample_pattern, RulePattern::Unknown | RulePattern::Complex { .. }) {
                pattern = sample_pattern;
            }
        }
    }

    Ok(Some(RectorRule {
        name,
        category,
        description: parsed.description.unwrap_or_default(),
        node_types: parsed.node_types,
        min_php_version,
        before_code: parsed.before_code.unwrap_or_default(),
        after_code: parsed.after_code.unwrap_or_default(),
        pattern,
        is_configurable: parsed.is_configurable,
        source_file: path.to_string_lossy().to_string(),
    }))
}

/// Extract rules from a specific category
pub fn extract_rules_from_category(repo_path: &Path, category: &str) -> ImportResult {
    let mut result = ImportResult::new();

    let category_path = repo_path.join("rules").join(category);
    if !category_path.exists() {
        result.warnings.push(format!(
            "Category directory not found: {}",
            category_path.display()
        ));
        return result;
    }

    for entry in WalkDir::new(&category_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map(|ext| ext == "php").unwrap_or(false)
                && e.path()
                    .file_name()
                    .map(|n| n.to_string_lossy().ends_with("Rector.php"))
                    .unwrap_or(false)
        })
    {
        let path = entry.path();

        match extract_rule_from_file(path) {
            Ok(Some(rule)) => result.rules.push(rule),
            Ok(None) => {
                result.warnings.push(format!(
                    "Could not extract rule from: {}",
                    path.display()
                ));
            }
            Err(e) => {
                result.failed.push((path.to_string_lossy().to_string(), e));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_rule(dir: &Path, content: &str) -> std::path::PathBuf {
        let file_path = dir.join("TestRector.php");
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    #[test]
    fn test_extract_simple_rule() {
        let temp = TempDir::new().unwrap();
        let content = r#"<?php
namespace Rector\CodeQuality\Rector\Identical;

use PhpParser\Node\Expr\FuncCall;
use Rector\Core\Rector\AbstractRector;

final class IsNullRector extends AbstractRector
{
    public function getNodeTypes(): array
    {
        return [FuncCall::class];
    }

    public function getRuleDefinition(): RuleDefinition
    {
        return new RuleDefinition('Replace is_null() with === null', [
            new CodeSample(
                <<<'CODE_SAMPLE'
is_null($value)
CODE_SAMPLE,
                <<<'CODE_SAMPLE'
$value === null
CODE_SAMPLE
            ),
        ]);
    }

    public function refactor(Node $node): ?Node
    {
        if (!$this->isName($node, 'is_null')) {
            return null;
        }
        return new Identical($node->args[0]->value, $this->nodeFactory->createNull());
    }
}
"#;

        let path = create_test_rule(temp.path(), content);
        let result = extract_rule_from_file(&path).unwrap();

        assert!(result.is_some());
        let rule = result.unwrap();
        assert_eq!(rule.name, "IsNullRector");
        assert_eq!(rule.description, "Replace is_null() with === null");
        assert_eq!(rule.node_types, vec!["FuncCall"]);
    }
}
