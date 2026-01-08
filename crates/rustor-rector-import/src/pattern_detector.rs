//! Pattern detector - analyzes refactor() body to detect rule patterns

use crate::RulePattern;
use regex::Regex;

/// Detect the pattern used in a Rector rule's refactor() method
pub fn detect_pattern(refactor_body: &str, node_types: &[String]) -> RulePattern {
    // Try each pattern detector in order of specificity
    // More specific patterns (known aliases, casts, comparisons) checked before generic rename

    // 1. Function alias: sizeof -> count (known aliases checked first)
    if let Some(pattern) = detect_function_alias(refactor_body) {
        return pattern;
    }

    // 2. Function to comparison: is_null -> $x === null
    if let Some(pattern) = detect_function_to_comparison(refactor_body) {
        return pattern;
    }

    // 3. Function to cast: strval -> (string)
    if let Some(pattern) = detect_function_to_cast(refactor_body) {
        return pattern;
    }

    // 4. Function to operator: pow -> **
    if let Some(pattern) = detect_function_to_operator(refactor_body) {
        return pattern;
    }

    // 5. Generic function rename pattern: isName + new Name
    if let Some(pattern) = detect_function_rename(refactor_body) {
        return pattern;
    }

    // 6. Array syntax modernization
    if node_types.contains(&"Array_".to_string()) || refactor_body.contains("ShortArraySyntax") {
        return RulePattern::ArraySyntaxModern;
    }

    // 7. Closure to arrow function
    if node_types.contains(&"Closure".to_string()) && refactor_body.contains("ArrowFunction") {
        return RulePattern::ClosureToArrow;
    }

    // 8. Ternary to coalesce
    if let Some(pattern) = detect_ternary_to_coalesce(refactor_body) {
        return pattern;
    }

    // Fall back to complex pattern with hints
    let hints = extract_pattern_hints(refactor_body);
    if !hints.is_empty() {
        return RulePattern::Complex {
            hints,
            refactor_body: refactor_body.to_string(),
        };
    }

    RulePattern::Unknown
}

/// Detect function rename pattern: join() -> implode()
fn detect_function_rename(body: &str) -> Option<RulePattern> {
    // Pattern: $this->isName($node, 'old_name') + $node->name = new Name('new_name')
    let is_name_re = Regex::new(r#"\$this->isName\s*\(\s*\$\w+\s*,\s*['"]([\w_]+)['"]"#).ok()?;
    let new_name_re = Regex::new(r#"new\s+Name\s*\(\s*['"](\w+)['"]"#).ok()?;

    let from = is_name_re
        .captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())?;

    let to = new_name_re
        .captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())?;

    // Make sure they're different
    if from != to {
        Some(RulePattern::FunctionRename { from, to })
    } else {
        None
    }
}

/// Detect function to comparison: is_null($x) -> $x === null
fn detect_function_to_comparison(body: &str) -> Option<RulePattern> {
    // Check for is_null pattern specifically
    if body.contains("is_null") || body.contains("'is_null'") {
        if body.contains("Identical") && body.contains("createNull") {
            return Some(RulePattern::FunctionToComparison {
                func: "is_null".to_string(),
                operator: "===".to_string(),
                compare_value: "null".to_string(),
            });
        }
        if body.contains("NotIdentical") && body.contains("createNull") {
            return Some(RulePattern::FunctionToComparison {
                func: "is_null".to_string(),
                operator: "!==".to_string(),
                compare_value: "null".to_string(),
            });
        }
    }

    // Check for is_array, is_string, etc.
    let type_check_re = Regex::new(r#"isName\s*\([^,]+,\s*['"]is_(\w+)['"]"#).ok()?;
    if let Some(caps) = type_check_re.captures(body) {
        let type_name = caps.get(1).map(|m| m.as_str())?;
        if body.contains("Instanceof") || body.contains("getType") {
            return Some(RulePattern::FunctionToComparison {
                func: format!("is_{}", type_name),
                operator: "instanceof".to_string(),
                compare_value: type_name.to_string(),
            });
        }
    }

    None
}

/// Detect function alias: sizeof -> count
fn detect_function_alias(body: &str) -> Option<RulePattern> {
    // Known aliases
    let aliases = [
        ("sizeof", "count"),
        ("join", "implode"),
        ("chop", "rtrim"),
        ("close", "closedir"),
        ("doubleval", "floatval"),
        ("fputs", "fwrite"),
        ("ini_alter", "ini_set"),
        ("is_double", "is_float"),
        ("is_integer", "is_int"),
        ("is_long", "is_int"),
        ("is_real", "is_float"),
        ("is_writeable", "is_writable"),
        ("key_exists", "array_key_exists"),
        ("pos", "current"),
        ("show_source", "highlight_file"),
        ("strchr", "strstr"),
    ];

    for (from, to) in aliases {
        // Check for 'from' with single or double quotes
        let has_from = body.contains(&format!("'{}'", from))
            || body.contains(&format!("\"{}\"", from));

        if has_from {
            // Check for 'to' with single or double quotes, or in Name() constructor
            // Handle both Name('x') and new Name('x')
            let has_to = body.contains(&format!("'{}'", to))
                || body.contains(&format!("\"{}\"", to));

            if has_to {
                return Some(RulePattern::FunctionAlias {
                    from: from.to_string(),
                    to: to.to_string(),
                });
            }
        }
    }

    None
}

/// Detect function to cast: strval -> (string)
fn detect_function_to_cast(body: &str) -> Option<RulePattern> {
    let casts = [
        ("strval", "string"),
        ("intval", "int"),
        ("floatval", "float"),
        ("boolval", "bool"),
    ];

    for (func, cast_type) in casts {
        if body.contains(&format!("'{}'", func)) || body.contains(&format!("\"{}\"", func)) {
            if body.contains("Cast") || body.contains(&format!("({})", cast_type)) {
                return Some(RulePattern::FunctionToCast {
                    func: func.to_string(),
                    cast_type: cast_type.to_string(),
                });
            }
        }
    }

    None
}

/// Detect function to operator: pow($x, 2) -> $x ** 2
fn detect_function_to_operator(body: &str) -> Option<RulePattern> {
    let func_to_op = [
        ("pow", "**"),
        // Could add more if Rector has them
    ];

    for (func, op) in func_to_op {
        if body.contains(&format!("'{}'", func)) || body.contains(&format!("\"{}\"", func)) {
            if body.contains("BinaryOp") || body.contains("Pow") {
                return Some(RulePattern::FunctionToOperator {
                    func: func.to_string(),
                    operator: op.to_string(),
                    arg_positions: vec![0, 1],
                });
            }
        }
    }

    None
}

/// Detect ternary to coalesce: isset($x) ? $x : $y -> $x ?? $y
fn detect_ternary_to_coalesce(body: &str) -> Option<RulePattern> {
    if body.contains("Ternary") || body.contains("ternary") {
        if body.contains("Coalesce") || body.contains("??") {
            if body.contains("isset") {
                return Some(RulePattern::TernaryToCoalesce {
                    condition_func: "isset".to_string(),
                });
            }
            if body.contains("empty") {
                return Some(RulePattern::TernaryToCoalesce {
                    condition_func: "empty".to_string(),
                });
            }
        }
    }
    None
}

/// Extract hints about what the pattern does
fn extract_pattern_hints(body: &str) -> Vec<String> {
    let mut hints = Vec::new();

    // Look for common method calls
    if body.contains("isName(") {
        hints.push("Uses name matching".to_string());
    }
    if body.contains("isObjectType(") {
        hints.push("Uses type checking".to_string());
    }
    if body.contains("getType(") {
        hints.push("Uses PHPStan type info".to_string());
    }
    if body.contains("traverseNodesWithCallable") {
        hints.push("Traverses child nodes".to_string());
    }
    if body.contains("removeNode") {
        hints.push("Removes nodes".to_string());
    }
    if body.contains("addNodeAfterNode") || body.contains("addNodeBeforeNode") {
        hints.push("Adds sibling nodes".to_string());
    }

    // Look for specific node types being created
    let node_creates = [
        ("new Identical", "Creates === comparison"),
        ("new NotIdentical", "Creates !== comparison"),
        ("new Coalesce", "Creates ?? expression"),
        ("new NullsafeMethodCall", "Creates ?-> call"),
        ("new ArrowFunction", "Creates arrow function"),
        ("new Match_", "Creates match expression"),
        ("new Attribute", "Creates PHP attribute"),
    ];

    for (pattern, hint) in node_creates {
        if body.contains(pattern) {
            hints.push(hint.to_string());
        }
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_function_rename() {
        // Use a function name that's NOT in the known alias list
        let body = r#"
            if (!$this->isName($node, 'old_function')) {
                return null;
            }
            $node->name = new Name('new_function');
            return $node;
        "#;
        let pattern = detect_pattern(body, &[]);
        assert!(matches!(
            pattern,
            RulePattern::FunctionRename { from, to } if from == "old_function" && to == "new_function"
        ));
    }

    #[test]
    fn test_detect_function_to_comparison() {
        let body = r#"
            if (!$this->isName($node, 'is_null')) {
                return null;
            }
            return new Identical($arg, $this->nodeFactory->createNull());
        "#;
        let pattern = detect_pattern(body, &[]);
        assert!(matches!(
            pattern,
            RulePattern::FunctionToComparison { func, operator, compare_value }
            if func == "is_null" && operator == "===" && compare_value == "null"
        ));
    }

    #[test]
    fn test_detect_function_alias() {
        let body = r#"
            if (!$this->isName($node, 'sizeof')) {
                return null;
            }
            return new FuncCall(new Name('count'), $node->args);
        "#;
        let pattern = detect_pattern(body, &[]);
        assert!(matches!(
            pattern,
            RulePattern::FunctionAlias { from, to } if from == "sizeof" && to == "count"
        ));
    }
}
