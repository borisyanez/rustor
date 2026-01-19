//! Pattern detector - analyzes refactor() body to detect rule patterns

use crate::ast_analyzer::{analyze_php, detect_pattern_from_ast, DetectedPattern};
use crate::RulePattern;
use regex::Regex;

/// Detect the pattern using AST analysis first, then fall back to regex
pub fn detect_pattern_with_ast(refactor_body: &str, node_types: &[String]) -> RulePattern {
    // Wrap the refactor body in a minimal PHP context for AST parsing
    let php_code = format!("<?php\nclass R {{\n    public function refactor() {{\n{}\n    }}\n}}", refactor_body);

    // Try AST-based detection first
    let analysis = analyze_php(&php_code);
    if let Some(detected) = detect_pattern_from_ast(&analysis) {
        match detected {
            DetectedPattern::FunctionRename { from, to } => {
                // Check if it's a known alias
                if is_known_alias(&from, &to) {
                    return RulePattern::FunctionAlias { from, to };
                }
                return RulePattern::FunctionRename { from, to };
            }
            DetectedPattern::FunctionToComparison {
                func,
                operator,
                compare_value,
            } => {
                return RulePattern::FunctionToComparison {
                    func,
                    operator,
                    compare_value,
                };
            }
            DetectedPattern::FunctionToCast { func, cast_type } => {
                return RulePattern::FunctionToCast { func, cast_type };
            }
            DetectedPattern::FunctionToOperator { func, operator } => {
                return RulePattern::FunctionToOperator {
                    func,
                    operator,
                    arg_positions: vec![0, 1],
                };
            }
            DetectedPattern::FunctionToClassConstant { func } => {
                return RulePattern::FunctionToClassConstant { func };
            }
            DetectedPattern::FunctionToInstanceof { func } => {
                return RulePattern::FunctionToInstanceof { func };
            }
            DetectedPattern::FunctionNoArgsToFunction { from, to } => {
                return RulePattern::FunctionNoArgsToFunction { from, to };
            }
            DetectedPattern::TernaryToCoalesce => {
                return RulePattern::TernaryToCoalesce {
                    condition_func: "isset".to_string(),
                };
            }
            DetectedPattern::StrContains => {
                return RulePattern::StrContains;
            }
            DetectedPattern::StrStartsWith => {
                return RulePattern::StrStartsWith;
            }
            DetectedPattern::StrEndsWith => {
                return RulePattern::StrEndsWith;
            }
        }
    }

    // Fall back to regex-based detection
    detect_pattern(refactor_body, node_types)
}

/// Check if a function rename is a known alias
fn is_known_alias(from: &str, to: &str) -> bool {
    let aliases = [
        ("sizeof", "count"),
        ("key_exists", "array_key_exists"),
        ("pos", "current"),
        ("join", "implode"),
        ("chop", "rtrim"),
        ("strchr", "strstr"),
        ("split", "preg_split"),
        ("spliti", "preg_split"),
        ("is_double", "is_float"),
        ("is_integer", "is_int"),
        ("is_long", "is_int"),
        ("is_real", "is_float"),
        ("is_writeable", "is_writable"),
        ("fputs", "fwrite"),
        ("close", "closedir"),
        ("show_source", "highlight_file"),
        ("doubleval", "floatval"),
        ("ini_alter", "ini_set"),
    ];

    aliases.iter().any(|(a, b)| a == &from && b == &to)
}

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

    // 15. Ternary to elvis: $a ? $a : $b -> $a ?: $b
    if let Some(pattern) = detect_ternary_to_elvis(refactor_body, node_types) {
        return pattern;
    }

    // 16. Function argument swap: array_key_exists($k, $arr) with type-checked $arr
    if let Some(pattern) = detect_function_arg_swap(refactor_body) {
        return pattern;
    }

    // 17. Comparison to function: strpos !== false -> str_contains
    if let Some(pattern) = detect_comparison_to_function(refactor_body) {
        return pattern;
    }

    // 18. str_starts_with pattern: substr($h, 0, strlen($n)) === $n
    if let Some(pattern) = detect_str_starts_with(refactor_body, node_types) {
        return pattern;
    }

    // 19. str_ends_with pattern: substr($h, -strlen($n)) === $n
    if let Some(pattern) = detect_str_ends_with(refactor_body, node_types) {
        return pattern;
    }

    // 20. str_contains pattern: strpos !== false or strstr
    if let Some(pattern) = detect_str_contains(refactor_body, node_types) {
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

/// Detect ternary to elvis: $a ? $a : $b -> $a ?: $b
fn detect_ternary_to_elvis(body: &str, node_types: &[String]) -> Option<RulePattern> {
    // Must operate on Ternary nodes
    if !node_types.contains(&"Ternary".to_string()) {
        return None;
    }

    // Pattern: areNodesEqual($node->cond, $node->if) and sets $node->if = null
    if body.contains("areNodesEqual") && body.contains("cond") && body.contains("->if") {
        if body.contains("->if = null") || body.contains("if = null") {
            return Some(RulePattern::TernaryToElvis);
        }
    }

    // Also check for elvis-specific keywords
    if body.contains("ELVIS") || body.contains("elvis") {
        return Some(RulePattern::TernaryToElvis);
    }

    None
}

/// Detect function with argument swap: array_key_exists($k, $obj) -> property_exists($obj, $k)
fn detect_function_arg_swap(body: &str) -> Option<RulePattern> {
    // Known function swaps
    let swaps = [
        ("array_key_exists", "property_exists", vec![1, 0]),
        // Add more as discovered
    ];

    for (from, to, order) in swaps {
        let has_from =
            body.contains(&format!("'{}'", from)) || body.contains(&format!("\"{}\"", from));
        let has_to = body.contains(&format!("'{}'", to)) || body.contains(&format!("\"{}\"", to));
        let has_reverse = body.contains("array_reverse") || body.contains("reverse");

        if has_from && has_to && has_reverse {
            return Some(RulePattern::FunctionArgSwap {
                func: from.to_string(),
                new_func: to.to_string(),
                arg_order: order,
            });
        }
    }

    None
}

/// Detect comparison to function: strpos !== false -> str_contains
fn detect_comparison_to_function(body: &str) -> Option<RulePattern> {
    // Known comparison to function conversions
    let conversions = [
        (
            "strpos",
            "str_contains",
            "!==",
            "false",
            false,
        ),
        (
            "strpos",
            "str_contains",
            "===",
            "false",
            true,
        ),
        (
            "strstr",
            "str_contains",
            "!==",
            "false",
            false,
        ),
    ];

    for (old_func, new_func, _op, _val, negate) in conversions {
        let has_old = body.contains(&format!("'{}'", old_func))
            || body.contains(&format!("\"{}\"", old_func));
        let has_new = body.contains(&format!("'{}'", new_func))
            || body.contains(&format!("\"{}\"", new_func));

        // Check for comparison pattern
        let has_comparison = body.contains("Identical")
            || body.contains("NotIdentical")
            || body.contains("=== false")
            || body.contains("!== false");

        if has_old && has_new && has_comparison {
            return Some(RulePattern::ComparisonToFunction {
                old_func: old_func.to_string(),
                new_func: new_func.to_string(),
                operator: "!==".to_string(),
                compare_value: "false".to_string(),
                negate_result: negate,
            });
        }
    }

    None
}

/// Detect str_starts_with pattern
/// Patterns:
/// - substr($h, 0, strlen($n)) === $n -> str_starts_with($h, $n)
/// - strpos($h, $n) === 0 -> str_starts_with($h, $n)
/// - strncmp($h, $n, strlen($n)) === 0 -> str_starts_with($h, $n)
fn detect_str_starts_with(body: &str, node_types: &[String]) -> Option<RulePattern> {
    // Must operate on comparison nodes
    let has_comparison_types = node_types.iter().any(|t| {
        t == "Identical" || t == "NotIdentical" || t == "Equal" || t == "NotEqual"
    });

    if !has_comparison_types && !node_types.is_empty() {
        return None;
    }

    // Check for str_starts_with in output
    let has_str_starts_with = body.contains("str_starts_with")
        || body.contains("'str_starts_with'")
        || body.contains("\"str_starts_with\"");

    if !has_str_starts_with {
        return None;
    }

    // Check for input patterns
    let has_substr_pattern = body.contains("substr") && body.contains("strlen");
    let has_strpos_zero = body.contains("strpos") && (body.contains("=== 0") || body.contains("== 0"));
    let has_strncmp = body.contains("strncmp");

    if has_substr_pattern || has_strpos_zero || has_strncmp {
        return Some(RulePattern::StrStartsWith);
    }

    None
}

/// Detect str_ends_with pattern
/// Pattern: substr($h, -strlen($n)) === $n -> str_ends_with($h, $n)
fn detect_str_ends_with(body: &str, node_types: &[String]) -> Option<RulePattern> {
    // Must operate on comparison nodes
    let has_comparison_types = node_types.iter().any(|t| {
        t == "Identical" || t == "NotIdentical" || t == "Equal" || t == "NotEqual"
    });

    if !has_comparison_types && !node_types.is_empty() {
        return None;
    }

    // Check for str_ends_with in output
    let has_str_ends_with = body.contains("str_ends_with")
        || body.contains("'str_ends_with'")
        || body.contains("\"str_ends_with\"");

    if !has_str_ends_with {
        return None;
    }

    // Check for substr with negative offset pattern
    let has_substr_negative = body.contains("substr") &&
        (body.contains("-strlen") || body.contains("- strlen") || body.contains("UnaryMinus"));

    // Check for substr_compare pattern
    let has_substr_compare = body.contains("substr_compare");

    if has_substr_negative || has_substr_compare {
        return Some(RulePattern::StrEndsWith);
    }

    None
}

/// Detect str_contains pattern
/// Pattern: strpos($h, $n) !== false -> str_contains($h, $n)
/// Pattern: strstr($h, $n) !== false -> str_contains($h, $n)
fn detect_str_contains(body: &str, node_types: &[String]) -> Option<RulePattern> {
    // Must operate on comparison nodes
    let has_comparison_types = node_types.iter().any(|t| {
        t == "Identical" || t == "NotIdentical" || t == "Equal" || t == "NotEqual"
    });

    if !has_comparison_types && !node_types.is_empty() {
        return None;
    }

    // Check for str_contains in output
    let has_str_contains = body.contains("str_contains")
        || body.contains("'str_contains'")
        || body.contains("\"str_contains\"");

    if !has_str_contains {
        return None;
    }

    // Check for strpos/strstr with false comparison
    let has_strpos_false = (body.contains("strpos") || body.contains("strstr")) &&
        (body.contains("false") || body.contains("FALSE"));

    if has_strpos_false {
        return Some(RulePattern::StrContains);
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

/// Detect pattern from code samples (before/after)
/// This is used as a fallback when refactor body analysis fails
pub fn detect_pattern_from_samples(
    before: &str,
    after: &str,
    node_types: &[String],
) -> RulePattern {
    // str_starts_with pattern
    // Before: substr($h, 0, strlen($n)) === $n  OR  strpos($h, $n) === 0
    // After: str_starts_with($h, $n)
    if after.contains("str_starts_with") {
        let before_lower = before.to_lowercase();
        let has_substr_strlen = before_lower.contains("substr") && before_lower.contains("strlen");
        let has_strpos_zero = before_lower.contains("strpos")
            && (before.contains("=== 0") || before.contains("== 0") || before.contains("!== 0") || before.contains("!= 0"));
        let has_strncmp = before_lower.contains("strncmp");

        if has_substr_strlen || has_strpos_zero || has_strncmp {
            return RulePattern::StrStartsWith;
        }
    }

    // str_ends_with pattern
    // Before: substr($h, -strlen($n)) === $n
    // After: str_ends_with($h, $n)
    if after.contains("str_ends_with") {
        let before_lower = before.to_lowercase();
        let has_substr_neg = before_lower.contains("substr") && before.contains("-");

        if has_substr_neg {
            return RulePattern::StrEndsWith;
        }
    }

    // str_contains pattern
    // Before: strpos($h, $n) !== false  OR  strstr($h, $n) !== false
    // After: str_contains($h, $n)
    if after.contains("str_contains") {
        let before_lower = before.to_lowercase();
        let has_strpos_false = (before_lower.contains("strpos") || before_lower.contains("strstr"))
            && (before.contains("false") || before.contains("FALSE"));

        if has_strpos_false {
            return RulePattern::StrContains;
        }
    }

    // Ternary to elvis: $a ? $a : $b -> $a ?: $b
    if after.contains("?:") && before.contains("?") && before.contains(":") && !before.contains("?:") {
        return RulePattern::TernaryToElvis;
    }

    // Nullsafe operator: $x !== null ? $x->y : null -> $x?->y
    if after.contains("?->") && !before.contains("?->") {
        return RulePattern::NullsafeMethodCall;
    }

    // Null coalesce: isset($x) ? $x : $y -> $x ?? $y
    if after.contains("??") && !before.contains("??") {
        if before.contains("isset") {
            return RulePattern::TernaryToCoalesce {
                condition_func: "isset".to_string(),
            };
        } else if before.contains("?") && before.contains(":") {
            return RulePattern::TernaryToCoalesce {
                condition_func: "ternary".to_string(),
            };
        }
    }

    // Closure to arrow: function() use ($x) { return $y; } -> fn() => $y
    if (after.contains("fn(") || after.contains("fn ("))
        && (before.contains("function") && before.contains("return"))
    {
        return RulePattern::ClosureToArrow;
    }

    // First-class callable: Closure::fromCallable([$obj, 'method']) -> $obj->method(...)
    if after.contains("...)")
        && (before.contains("fromCallable") || before.contains("Closure::fromCallable"))
    {
        return RulePattern::FirstClassCallable;
    }

    // pow to ** operator
    if after.contains("**") && before.to_lowercase().contains("pow(") {
        return RulePattern::FunctionToOperator {
            func: "pow".to_string(),
            operator: "**".to_string(),
            arg_positions: vec![0, 1], // pow($base, $exp) -> $base ** $exp
        };
    }

    // Check for comparison type requirements
    let has_comparison_types = node_types.iter().any(|t| {
        t == "Identical" || t == "NotIdentical" || t == "Equal" || t == "NotEqual"
    });

    // Fall back to complex if we have hints
    if has_comparison_types || !node_types.is_empty() {
        return RulePattern::Complex {
            hints: vec!["Detected from code samples".to_string()],
            refactor_body: format!("Before: {}\nAfter: {}", before, after),
        };
    }

    RulePattern::Unknown
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

    #[test]
    fn test_detect_ternary_to_elvis() {
        let body = r#"
            if (!$this->nodeComparator->areNodesEqual($node->cond, $node->if)) {
                return null;
            }
            $node->if = null;
            return $node;
        "#;
        let pattern = detect_pattern(body, &["Ternary".to_string()]);
        assert!(matches!(pattern, RulePattern::TernaryToElvis));
    }

    #[test]
    fn test_detect_comparison_to_function() {
        // Test body that matches comparison-to-function pattern without matching FunctionRename
        let body = r#"
            // Converts strpos($h, $n) !== false to str_contains($h, $n)
            // Uses Identical or NotIdentical nodes for comparison
            if ($node instanceof NotIdentical && $this->isFalseLiteral($node->right)) {
                $funcCall = $this->createStrContainsCall('strpos', 'str_contains');
                return $funcCall;
            }
        "#;
        let pattern = detect_pattern(body, &[]);
        assert!(matches!(
            pattern,
            RulePattern::ComparisonToFunction { old_func, new_func, .. }
            if old_func == "strpos" && new_func == "str_contains"
        ));
    }
}
