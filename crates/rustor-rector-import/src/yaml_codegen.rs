//! YAML code generator - converts Rector rules to rustor YAML rules
//!
//! Generates YAML rules that can be loaded by rustor-rules yaml_rules engine.

use crate::{RectorRule, RulePattern};

/// Generate YAML rule from a RectorRule
pub fn generate_yaml_rule(rule: &RectorRule) -> Option<String> {
    let pattern = &rule.pattern;

    // Only generate for auto-generatable patterns
    if !pattern.is_auto_generatable() {
        return None;
    }

    let (match_pattern, replace) = match pattern {
        RulePattern::FunctionAlias { from, to } => {
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $args..."#,
                from
            );
            let replace = format!("{}($args)", to);
            (match_pat, replace)
        }

        RulePattern::FunctionRename { from, to } => {
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $args..."#,
                from
            );
            let replace = format!("{}($args)", to);
            (match_pat, replace)
        }

        RulePattern::FunctionToComparison { func, operator, compare_value } => {
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $expr"#,
                func
            );
            let replace = format!("$expr {} {}", operator, compare_value);
            (match_pat, replace)
        }

        RulePattern::FunctionToCast { func, cast_type } => {
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $expr"#,
                func
            );
            let replace = format!("({}){}", cast_type, "$expr");
            (match_pat, replace)
        }

        RulePattern::FunctionToOperator { func, operator, arg_positions } => {
            if arg_positions.len() != 2 {
                return None;
            }
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $left
    - capture: $right"#,
                func
            );
            let replace = format!("$left {} $right", operator);
            (match_pat, replace)
        }

        RulePattern::FunctionToClassConstant { func } => {
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $obj"#,
                func
            );
            let replace = "$obj::class".to_string();
            (match_pat, replace)
        }

        RulePattern::FunctionToInstanceof { func } => {
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $obj
    - capture: $class"#,
                func
            );
            let replace = "$obj instanceof $class".to_string();
            (match_pat, replace)
        }

        RulePattern::UnwrapSingleArgFunction { func } => {
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $arg
    - no_more: true"#,
                func
            );
            let replace = "$arg".to_string();
            (match_pat, replace)
        }

        RulePattern::FunctionNoArgsToFunction { from, to } => {
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args: []"#,
                from
            );
            let replace = format!("{}()", to);
            (match_pat, replace)
        }

        RulePattern::FunctionRemoveFirstArg { func } => {
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $_first
    - capture: $rest..."#,
                func
            );
            let replace = format!("{}($rest)", func);
            (match_pat, replace)
        }

        RulePattern::FunctionArgSwap { func, new_func, arg_order } => {
            if arg_order.len() != 2 {
                return None;
            }
            let match_pat = format!(
                r#"match:
  node: FuncCall
  name: {}
  args:
    - capture: $arg0
    - capture: $arg1"#,
                func
            );
            // Reorder based on arg_order
            let (first, second) = if arg_order[0] == 1 { ("$arg1", "$arg0") } else { ("$arg0", "$arg1") };
            let replace = format!("{}({}, {})", new_func, first, second);
            (match_pat, replace)
        }

        RulePattern::TernaryToCoalesce { condition_func } => {
            let match_pat = if condition_func == "isset" {
                r#"match:
  node: Ternary
  condition:
    node: Isset
    args:
      - capture: $var
  then:
    same_as: $var
  else:
    capture: $default"#.to_string()
            } else {
                // Generic ternary pattern
                r#"match:
  node: Ternary
  condition:
    capture: $cond
  then:
    same_as: $cond
  else:
    capture: $default"#.to_string()
            };
            let replace = "$var ?? $default".to_string();
            (match_pat, replace)
        }

        RulePattern::TernaryToElvis => {
            let match_pat = r#"match:
  node: Ternary
  condition:
    capture: $cond
  then:
    same_as: $cond
  else:
    capture: $default"#.to_string();
            let replace = "$cond ?: $default".to_string();
            (match_pat, replace)
        }

        RulePattern::ComparisonToFunction { old_func, new_func, operator, compare_value, negate_result } => {
            let match_pat = format!(
                r#"match:
  node: BinaryOp
  operator: "{}"
  left:
    node: FuncCall
    name: {}
    args:
      - capture: $haystack
      - capture: $needle
  right:
    node: Literal{}"#,
                operator,
                old_func,
                capitalize(compare_value)
            );
            let replace = if *negate_result {
                format!("!{}($haystack, $needle)", new_func)
            } else {
                format!("{}($haystack, $needle)", new_func)
            };
            (match_pat, replace)
        }

        RulePattern::StrStartsWith => {
            // strpos($haystack, $needle) === 0 -> str_starts_with($haystack, $needle)
            let match_pat = r#"match:
  any:
    - node: BinaryOp
      operator: "==="
      left:
        node: FuncCall
        name: strpos
        args:
          - capture: $haystack
          - capture: $needle
      right:
        node: LiteralInt
        value: 0"#.to_string();
            let replace = "str_starts_with($haystack, $needle)".to_string();
            (match_pat, replace)
        }

        RulePattern::StrEndsWith => {
            // Complex pattern, generate placeholder
            let match_pat = r#"match:
  # str_ends_with pattern - requires complex substr/strlen matching
  node: BinaryOp
  operator: "==="
  left:
    capture: $substr_expr
  right:
    capture: $needle"#.to_string();
            let replace = "str_ends_with($haystack, $needle)".to_string();
            (match_pat, replace)
        }

        RulePattern::StrContains => {
            let match_pat = r#"match:
  any:
    - node: BinaryOp
      operator: "!=="
      left:
        node: FuncCall
        name: strpos
        args:
          - capture: $haystack
          - capture: $needle
      right:
        node: LiteralFalse
    - node: BinaryOp
      operator: "!="
      left:
        node: FuncCall
        name: strpos
        args:
          - capture: $haystack
          - capture: $needle
      right:
        node: LiteralFalse"#.to_string();
            let replace = "str_contains($haystack, $needle)".to_string();
            (match_pat, replace)
        }

        RulePattern::ArraySyntaxModern => {
            let match_pat = r#"match:
  node: Array
  syntax: long"#.to_string();
            let replace = "[$items]".to_string();
            (match_pat, replace)
        }

        RulePattern::ClosureToArrow => {
            // Complex pattern - needs proper closure matching
            let match_pat = r#"match:
  # Closure to arrow function - requires closure analysis
  node: Closure
  has_single_return: true"#.to_string();
            let replace = "fn($params) => $expr".to_string();
            (match_pat, replace)
        }

        RulePattern::NullsafeMethodCall => {
            // Complex pattern requiring literal null matching which the current schema doesn't support
            // TODO: Extend schema to support literal matching in nested conditions
            return None;
        }

        RulePattern::FirstClassCallable => {
            let match_pat = r#"match:
  node: StaticCall
  class: Closure
  method: fromCallable
  args:
    - capture: $callable"#.to_string();
            let replace = "$callable(...)".to_string();
            (match_pat, replace)
        }

        RulePattern::Complex { .. } | RulePattern::Unknown => {
            return None;
        }
    };

    // Generate the full YAML
    let name = to_snake_case(&rule.name.replace("Rector", ""));
    let category = map_category(&rule.category);
    let min_php = rule.min_php_version.as_deref().unwrap_or("5.4");

    // Escape description for YAML (truncate and escape)
    let description: String = rule.description
        .replace('\n', " ")
        .chars()
        .take(200)
        .collect();
    let description = escape_yaml_string(&description);

    // Build test cases from before/after code
    let tests = generate_tests(&rule.before_code, &rule.after_code);

    // Escape the replace string for YAML
    let replace_escaped = escape_yaml_string(&replace);

    let yaml = format!(
        r#"# Auto-generated from Rector: {}
# Source: {}
name: {}
description: "{}"
category: {}
min_php: "{}"

{}

replace: "{}"

{}
"#,
        rule.name,
        rule.source_file,
        name,
        description,
        category,
        min_php,
        match_pattern,
        replace_escaped,
        tests
    );

    Some(yaml)
}

/// Convert CamelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

/// Map Rector category to rustor category
fn map_category(category: &str) -> &'static str {
    match category.to_lowercase().as_str() {
        "codequality" | "code_quality" => "code_quality",
        "deadcode" | "dead_code" => "code_quality",
        "earlyreturn" | "early_return" => "code_quality",
        "strict" => "code_quality",
        "php70" | "php71" | "php72" | "php73" | "php74" => "modernization",
        "php80" | "php81" | "php82" | "php83" | "php84" => "modernization",
        "naming" | "renaming" => "compatibility",
        "transform" => "modernization",
        "typedeclaration" | "type_declaration" => "modernization",
        _ => "code_quality",
    }
}

/// Capitalize first letter
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Escape a string for YAML double-quoted format
fn escape_yaml_string(s: &str) -> String {
    s.replace('\\', "\\\\")  // Must escape backslashes first
     .replace('"', "\\\"")   // Escape double quotes
     .replace('\n', "\\n")   // Escape newlines
     .replace('\t', "\\t")   // Escape tabs
}

/// Generate test cases from before/after code samples
fn generate_tests(before: &str, after: &str) -> String {
    // Clean up code samples
    let before_clean = clean_code_sample(before);
    let after_clean = clean_code_sample(after);

    if before_clean.is_empty() || after_clean.is_empty() {
        return "tests: []".to_string();
    }

    // Try to extract the core expression
    let before_expr = extract_expression(&before_clean);
    let after_expr = extract_expression(&after_clean);

    if before_expr.is_empty() || after_expr.is_empty() {
        return "tests: []".to_string();
    }

    // Escape for YAML double-quoted strings (backslash must be doubled)
    let before_escaped = escape_yaml_string(&before_expr);
    let after_escaped = escape_yaml_string(&after_expr);

    format!(
        r#"tests:
  - input: "{}"
    output: "{}""#,
        before_escaped,
        after_escaped
    )
}

/// Clean up code sample (remove PHP tags, extra whitespace)
fn clean_code_sample(code: &str) -> String {
    code.trim()
        .trim_start_matches("<?php")
        .trim_start_matches("<?")
        .trim_end_matches("?>")
        .trim()
        .to_string()
}

/// Extract the core expression from a code sample
fn extract_expression(code: &str) -> String {
    // Try to find a simple expression (single line, no class definitions)
    let lines: Vec<&str> = code.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter(|l| !l.starts_with("//"))
        .filter(|l| !l.starts_with("class "))
        .filter(|l| !l.starts_with("function "))
        .filter(|l| !l.starts_with("namespace "))
        .filter(|l| !l.starts_with("use "))
        .collect();

    if lines.len() == 1 {
        return lines[0].trim_end_matches(';').to_string();
    }

    // If multiple lines, try to find the key expression
    for line in &lines {
        // Look for function calls, comparisons, etc.
        if line.contains('(') && !line.starts_with("if") && !line.starts_with("while") {
            return line.trim_end_matches(';').to_string();
        }
    }

    // Fallback: return first meaningful line
    lines.first()
        .map(|s| s.trim_end_matches(';').to_string())
        .unwrap_or_default()
}

/// Generate multiple YAML rules and combine into output
pub fn generate_yaml_rules(rules: &[RectorRule]) -> Vec<(String, String)> {
    rules.iter()
        .filter_map(|rule| {
            let yaml = generate_yaml_rule(rule)?;
            let filename = format!("{}.yaml", to_snake_case(&rule.name.replace("Rector", "")));
            Some((filename, yaml))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("IsNull"), "is_null");
        assert_eq!(to_snake_case("JoinToImplode"), "join_to_implode");
        assert_eq!(to_snake_case("HTMLParser"), "h_t_m_l_parser");
    }

    #[test]
    fn test_generate_function_alias() {
        let rule = RectorRule {
            name: "JoinToImplodeRector".to_string(),
            category: "CodeQuality".to_string(),
            description: "Convert join() to implode()".to_string(),
            node_types: vec!["FuncCall".to_string()],
            min_php_version: None,
            before_code: "<?php join(',', $arr);".to_string(),
            after_code: "<?php implode(',', $arr);".to_string(),
            pattern: RulePattern::FunctionAlias {
                from: "join".to_string(),
                to: "implode".to_string(),
            },
            is_configurable: false,
            source_file: "test.php".to_string(),
        };

        let yaml = generate_yaml_rule(&rule);
        assert!(yaml.is_some());
        let yaml = yaml.unwrap();
        assert!(yaml.contains("name: join_to_implode"));
        assert!(yaml.contains("node: FuncCall"));
        assert!(yaml.contains("name: join"));
        assert!(yaml.contains("implode($args)"));
    }

    #[test]
    fn test_generate_function_to_comparison() {
        let rule = RectorRule {
            name: "IsNullRector".to_string(),
            category: "CodeQuality".to_string(),
            description: "Convert is_null() to === null".to_string(),
            node_types: vec!["FuncCall".to_string()],
            min_php_version: None,
            before_code: "<?php is_null($x);".to_string(),
            after_code: "<?php $x === null;".to_string(),
            pattern: RulePattern::FunctionToComparison {
                func: "is_null".to_string(),
                operator: "===".to_string(),
                compare_value: "null".to_string(),
            },
            is_configurable: false,
            source_file: "test.php".to_string(),
        };

        let yaml = generate_yaml_rule(&rule);
        assert!(yaml.is_some());
        let yaml = yaml.unwrap();
        assert!(yaml.contains("name: is_null"));
        assert!(yaml.contains("$expr === null"));
    }
}
