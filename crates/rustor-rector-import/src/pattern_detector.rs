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

    // 2. Function to instanceof: is_a($x, Class::class) -> $x instanceof Class
    // Must be checked before function_to_comparison to avoid false matches on is_a
    if let Some(pattern) = detect_function_to_instanceof(refactor_body) {
        return pattern;
    }

    // 3. Function to comparison: is_null -> $x === null
    if let Some(pattern) = detect_function_to_comparison(refactor_body) {
        return pattern;
    }

    // 4. Function to cast: strval -> (string)
    if let Some(pattern) = detect_function_to_cast(refactor_body) {
        return pattern;
    }

    // 5. Function to operator: pow -> **
    if let Some(pattern) = detect_function_to_operator(refactor_body) {
        return pattern;
    }

    // 6. Function to ::class constant: get_class($x) -> $x::class
    if let Some(pattern) = detect_function_to_class_constant(refactor_body) {
        return pattern;
    }

    // 7. Unwrap single-arg function: sprintf($x) -> $x
    if let Some(pattern) = detect_unwrap_single_arg_function(refactor_body) {
        return pattern;
    }

    // 8. Function no args to another: mktime() -> time()
    if let Some(pattern) = detect_function_no_args_to_function(refactor_body) {
        return pattern;
    }

    // 9. Generic function rename pattern: isName + new Name
    if let Some(pattern) = detect_function_rename(refactor_body) {
        return pattern;
    }

    // 10. Array syntax modernization
    if node_types.contains(&"Array_".to_string()) || refactor_body.contains("ShortArraySyntax") {
        return RulePattern::ArraySyntaxModern;
    }

    // 11. Closure to arrow function
    if node_types.contains(&"Closure".to_string()) && refactor_body.contains("ArrowFunction") {
        return RulePattern::ClosureToArrow;
    }

    // 12. First-class callable syntax
    if refactor_body.contains("fromCallable") && refactor_body.contains("FirstClassCallable") {
        return RulePattern::FirstClassCallable;
    }

    // 13. Nullsafe method call
    if refactor_body.contains("NullsafeMethodCall") || refactor_body.contains("nullsafe") {
        return RulePattern::NullsafeMethodCall;
    }

    // 14. Ternary to coalesce
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
    // Known aliases - comprehensive list of PHP function aliases
    let aliases = [
        // Array functions
        ("sizeof", "count"),
        ("key_exists", "array_key_exists"),
        ("pos", "current"),
        // String functions
        ("join", "implode"),
        ("chop", "rtrim"),
        ("strchr", "strstr"),
        ("split", "preg_split"),
        ("spliti", "preg_split"),
        // Type check functions
        ("is_double", "is_float"),
        ("is_integer", "is_int"),
        ("is_long", "is_int"),
        ("is_real", "is_float"),
        ("is_writeable", "is_writable"),
        // I/O functions
        ("fputs", "fwrite"),
        ("close", "closedir"),
        ("show_source", "highlight_file"),
        // Value conversion
        ("doubleval", "floatval"),
        // Config functions
        ("ini_alter", "ini_set"),
        // Multibyte string functions
        ("mb_substr", "substr"),
        ("mb_strpos", "strpos"),
        // Other aliases
        ("set_file_buffer", "stream_set_write_buffer"),
        ("socket_get_status", "stream_get_meta_data"),
        ("socket_set_blocking", "stream_set_blocking"),
        ("socket_set_timeout", "stream_set_timeout"),
        ("mysql_escape_string", "mysqli_real_escape_string"),
        ("ereg", "preg_match"),
        ("eregi", "preg_match"),
        ("ereg_replace", "preg_replace"),
        ("eregi_replace", "preg_replace"),
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
        ("doubleval", "float"),
        ("boolval", "bool"),
        ("settype", "various"),
    ];

    for (func, cast_type) in casts {
        if body.contains(&format!("'{}'", func)) || body.contains(&format!("\"{}\"", func)) {
            if body.contains("Cast")
                || body.contains(&format!("({})", cast_type))
                || body.contains("String_")
                || body.contains("Int_")
                || body.contains("Double")
                || body.contains("Bool_")
            {
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

/// Detect function to ::class constant: get_class($x) -> $x::class
fn detect_function_to_class_constant(body: &str) -> Option<RulePattern> {
    // Pattern: isName('get_class') and creates ClassConstFetch with 'class'
    if body.contains("get_class") || body.contains("'get_class'") {
        if body.contains("ClassConstFetch") || body.contains("::class") {
            return Some(RulePattern::FunctionToClassConstant {
                func: "get_class".to_string(),
            });
        }
    }
    None
}

/// Detect function to instanceof: is_a($x, Class::class) -> $x instanceof Class
fn detect_function_to_instanceof(body: &str) -> Option<RulePattern> {
    // Pattern: isName('is_a') and creates Instanceof node
    if body.contains("'is_a'") || body.contains("\"is_a\"") {
        if body.contains("Instanceof") {
            return Some(RulePattern::FunctionToInstanceof {
                func: "is_a".to_string(),
            });
        }
    }
    None
}

/// Detect unwrap single-arg function: sprintf($x) -> $x
fn detect_unwrap_single_arg_function(body: &str) -> Option<RulePattern> {
    // Pattern: returns the first argument directly (unwraps the function)
    // Common patterns: sprintf with 1 arg, trim with 1 arg, etc.
    let unwrap_funcs = [
        ("sprintf", "getArgs", "count($node->args) === 1"),
        ("trim", "args", "count"),
        ("addslashes", "args", "count"),
        ("stripslashes", "args", "count"),
    ];

    for (func, _, count_check) in unwrap_funcs {
        let has_func =
            body.contains(&format!("'{}'", func)) || body.contains(&format!("\"{}\"", func));
        let has_single_arg = body.contains("getArgs()[0]")
            || body.contains("args[0]")
            || body.contains(count_check);
        let returns_arg = body.contains("return $node->args[0]")
            || body.contains("return $node->getArgs()[0]")
            || body.contains("->value");

        if has_func && has_single_arg && returns_arg {
            return Some(RulePattern::UnwrapSingleArgFunction {
                func: func.to_string(),
            });
        }
    }
    None
}

/// Detect function no args to another function: mktime() -> time()
fn detect_function_no_args_to_function(body: &str) -> Option<RulePattern> {
    // Pattern: check for no args, rename to different function
    let conversions = [
        ("mktime", "time"),
        ("gmmktime", "time"),
        ("restore_include_path", "ini_restore"),
    ];

    for (from, to) in conversions {
        let has_from =
            body.contains(&format!("'{}'", from)) || body.contains(&format!("\"{}\"", from));
        let has_to = body.contains(&format!("'{}'", to)) || body.contains(&format!("\"{}\"", to));
        let checks_no_args = body.contains("args") && body.contains("count")
            || body.contains("getArgs()")
            || body.contains("args === []")
            || body.contains("args === null");

        if has_from && has_to && checks_no_args {
            return Some(RulePattern::FunctionNoArgsToFunction {
                from: from.to_string(),
                to: to.to_string(),
            });
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

    #[test]
    fn test_detect_function_to_class_constant() {
        let body = r#"
            if (!$this->isName($node, 'get_class')) {
                return null;
            }
            return new ClassConstFetch($arg, new Identifier('class'));
        "#;
        let pattern = detect_pattern(body, &[]);
        assert!(matches!(
            pattern,
            RulePattern::FunctionToClassConstant { func } if func == "get_class"
        ));
    }

    #[test]
    fn test_detect_function_to_instanceof() {
        let body = r#"
            if (!$this->isName($node, 'is_a')) {
                return null;
            }
            return new Instanceof($firstArg, $secondArg);
        "#;
        let pattern = detect_pattern(body, &[]);
        assert!(matches!(
            pattern,
            RulePattern::FunctionToInstanceof { func } if func == "is_a"
        ));
    }

    #[test]
    fn test_detect_function_no_args_to_function() {
        let body = r#"
            if (!$this->isName($node, 'mktime')) {
                return null;
            }
            if (count($node->args) !== 0) {
                return null;
            }
            return new FuncCall(new Name('time'));
        "#;
        let pattern = detect_pattern(body, &[]);
        assert!(matches!(
            pattern,
            RulePattern::FunctionNoArgsToFunction { from, to } if from == "mktime" && to == "time"
        ));
    }

    #[test]
    fn test_detect_closure_to_arrow() {
        let body = r#"
            return new ArrowFunction([
                'params' => $node->params,
                'expr' => $expr,
            ]);
        "#;
        let pattern = detect_pattern(body, &["Closure".to_string()]);
        assert!(matches!(pattern, RulePattern::ClosureToArrow));
    }

    #[test]
    fn test_detect_first_class_callable() {
        let body = r#"
            if ($this->isName($node->class, 'Closure') && $this->isName($node->name, 'fromCallable')) {
                return new FirstClassCallable();
            }
        "#;
        let pattern = detect_pattern(body, &[]);
        assert!(matches!(pattern, RulePattern::FirstClassCallable));
    }

    #[test]
    fn test_detect_nullsafe_method_call() {
        let body = r#"
            return new NullsafeMethodCall($var, $node->name, $node->args);
        "#;
        let pattern = detect_pattern(body, &[]);
        assert!(matches!(pattern, RulePattern::NullsafeMethodCall));
    }
}
