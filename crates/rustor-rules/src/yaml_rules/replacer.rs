//! Replacement engine for YAML rules
//!
//! Handles template substitution and generates replacement PHP code
//! from captured bindings.

use regex::Regex;
use std::sync::OnceLock;

use super::matcher::CapturedBindings;
use super::schema::{Replacement, ReplacementNode};

/// Replacer that generates PHP code from templates and bindings
pub struct Replacer;

impl Replacer {
    /// Apply a replacement using the captured bindings
    pub fn apply(replacement: &Replacement, bindings: &CapturedBindings) -> Option<String> {
        match replacement {
            Replacement::Template(template) => Some(Self::substitute_template(template, bindings)),
            Replacement::Node(node) => Self::build_node(node, bindings),
            Replacement::Conditional(cond) => {
                // Evaluate condition and choose branch
                if Self::evaluate_condition(&cond.condition.condition, bindings) {
                    Self::apply(&cond.then_replace, bindings)
                } else {
                    Self::apply(&cond.else_replace, bindings)
                }
            }
            Replacement::Multiple { multiple } => {
                let results: Vec<_> = multiple
                    .iter()
                    .map(|t| Self::substitute_template(t, bindings))
                    .collect();
                Some(results.join("\n"))
            }
            Replacement::Remove => None, // Signals removal
        }
    }

    /// Substitute variables in a template string
    pub fn substitute_template(template: &str, bindings: &CapturedBindings) -> String {
        static VAR_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = VAR_REGEX.get_or_init(|| {
            // Match $varname or ${varname} patterns
            Regex::new(r"\$\{?([a-zA-Z_][a-zA-Z0-9_]*)\}?").unwrap()
        });

        let result = regex.replace_all(template, |caps: &regex::Captures| {
            let var_name = &caps[1];
            if let Some(value) = bindings.get_text(var_name) {
                value.to_string()
            } else {
                // Keep original if not found
                caps[0].to_string()
            }
        });

        result.into_owned()
    }

    /// Build a structured replacement node
    fn build_node(node: &ReplacementNode, bindings: &CapturedBindings) -> Option<String> {
        match node.node.as_str() {
            "FuncCall" => {
                let name = node.name.as_ref()?;
                let name = Self::substitute_template(name, bindings);
                let args: Vec<_> = node
                    .args
                    .iter()
                    .map(|a| Self::substitute_template(a, bindings))
                    .collect();
                Some(format!("{}({})", name, args.join(", ")))
            }
            "MethodCall" => {
                // $object->method(args)
                let obj = node.expr.as_ref().map(|e| Self::substitute_template(e, bindings))?;
                let method = node.name.as_ref()?;
                let method = Self::substitute_template(method, bindings);
                let args: Vec<_> = node
                    .args
                    .iter()
                    .map(|a| Self::substitute_template(a, bindings))
                    .collect();
                Some(format!("{}->{}({})", obj, method, args.join(", ")))
            }
            "StaticCall" => {
                // Class::method(args)
                let class = node.name.as_ref()?; // Use name for class
                let class = Self::substitute_template(class, bindings);
                let method = node.expr.as_ref().map(|e| Self::substitute_template(e, bindings))?;
                let args: Vec<_> = node
                    .args
                    .iter()
                    .map(|a| Self::substitute_template(a, bindings))
                    .collect();
                Some(format!("{}::{}({})", class, method, args.join(", ")))
            }
            "BinaryOp" => {
                let left = node.left.as_ref()?;
                let right = node.right.as_ref()?;
                let op = node.operator.as_ref()?;
                let left = Self::substitute_template(left, bindings);
                let right = Self::substitute_template(right, bindings);
                Some(format!("{} {} {}", left, op, right))
            }
            "Null" => Some("null".to_string()),
            "True" | "LiteralTrue" => Some("true".to_string()),
            "False" | "LiteralFalse" => Some("false".to_string()),
            "BooleanNot" => {
                let expr = node.expr.as_ref()?;
                let expr = Self::substitute_template(expr, bindings);
                Some(format!("!{}", expr))
            }
            "NullCoalesce" => {
                let left = node.left.as_ref()?;
                let right = node.right.as_ref()?;
                let left = Self::substitute_template(left, bindings);
                let right = Self::substitute_template(right, bindings);
                Some(format!("{} ?? {}", left, right))
            }
            "Ternary" => {
                let cond = node.expr.as_ref()?; // condition
                let left = node.left.as_ref()?; // then
                let right = node.right.as_ref()?; // else
                let cond = Self::substitute_template(cond, bindings);
                let left = Self::substitute_template(left, bindings);
                let right = Self::substitute_template(right, bindings);
                Some(format!("{} ? {} : {}", cond, left, right))
            }
            "Elvis" => {
                // $x ?: $default
                let left = node.left.as_ref()?;
                let right = node.right.as_ref()?;
                let left = Self::substitute_template(left, bindings);
                let right = Self::substitute_template(right, bindings);
                Some(format!("{} ?: {}", left, right))
            }
            "Cast" => {
                // (type)$expr
                let cast_type = node.name.as_ref()?;
                let expr = node.expr.as_ref()?;
                let expr = Self::substitute_template(expr, bindings);
                Some(format!("({}){}", cast_type, expr))
            }
            "Array" => {
                // [$arg1, $arg2, ...]
                let items: Vec<_> = node
                    .args
                    .iter()
                    .map(|a| Self::substitute_template(a, bindings))
                    .collect();
                Some(format!("[{}]", items.join(", ")))
            }
            "ArrayPush" => {
                // $arr[] = $value
                let arr = node.left.as_ref()?;
                let value = node.right.as_ref()?;
                let arr = Self::substitute_template(arr, bindings);
                let value = Self::substitute_template(value, bindings);
                Some(format!("{}[] = {}", arr, value))
            }
            "Instanceof" => {
                let left = node.left.as_ref()?;
                let right = node.right.as_ref()?;
                let left = Self::substitute_template(left, bindings);
                let right = Self::substitute_template(right, bindings);
                Some(format!("{} instanceof {}", left, right))
            }
            "ClassConstFetch" => {
                // ClassName::class
                let class = node.name.as_ref()?;
                let class = Self::substitute_template(class, bindings);
                Some(format!("{}::class", class))
            }
            _ => {
                // Unknown node type, try to substitute as template
                if let Some(expr) = &node.expr {
                    Some(Self::substitute_template(expr, bindings))
                } else {
                    None
                }
            }
        }
    }

    /// Evaluate a simple condition expression
    fn evaluate_condition(condition: &str, bindings: &CapturedBindings) -> bool {
        // Parse simple conditions like "$len.value > 0"
        // For now, support basic patterns:
        // - "$var.value > N" / "$var.value < N" / "$var.value == N"
        // - "$var.exists"
        // - "$var.type == typename"

        let condition = condition.trim();

        // Check for .exists
        if condition.ends_with(".exists") {
            let var = condition.strip_suffix(".exists").unwrap();
            let var = var.strip_prefix('$').unwrap_or(var);
            return bindings.contains(var);
        }

        // Check for .value comparisons
        if condition.contains(".value") {
            // Extract variable name and comparison
            static VALUE_REGEX: OnceLock<Regex> = OnceLock::new();
            let regex = VALUE_REGEX.get_or_init(|| {
                Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)\.value\s*(==|!=|>|<|>=|<=)\s*(.+)").unwrap()
            });

            if let Some(caps) = regex.captures(condition) {
                let var_name = &caps[1];
                let op = &caps[2];
                let expected = caps[3].trim();

                if let Some(value) = bindings.get_text(var_name) {
                    // Try to parse as number for numeric comparison
                    if let Ok(num_value) = value.parse::<i64>() {
                        if let Ok(num_expected) = expected.parse::<i64>() {
                            return match op {
                                "==" => num_value == num_expected,
                                "!=" => num_value != num_expected,
                                ">" => num_value > num_expected,
                                "<" => num_value < num_expected,
                                ">=" => num_value >= num_expected,
                                "<=" => num_value <= num_expected,
                                _ => false,
                            };
                        }
                    }
                    // String comparison
                    return match op {
                        "==" => value == expected,
                        "!=" => value != expected,
                        _ => false,
                    };
                }
            }
        }

        // Check for .type comparisons
        if condition.contains(".type") {
            // Type checking would require PHPStan integration
            // For now, just return true to not block the rule
            return true;
        }

        // Check for regex matches
        if condition.contains("matches(") {
            // matches(/pattern/)
            static MATCHES_REGEX: OnceLock<Regex> = OnceLock::new();
            let regex = MATCHES_REGEX.get_or_init(|| {
                Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)\.value:\s*matches\(/(.+)/\)").unwrap()
            });

            if let Some(caps) = regex.captures(condition) {
                let var_name = &caps[1];
                let pattern = &caps[2];

                if let Some(value) = bindings.get_text(var_name) {
                    if let Ok(re) = Regex::new(pattern) {
                        return re.is_match(value);
                    }
                }
            }
        }

        // Default to true for unknown conditions
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mago_database::file::FileId;
    use mago_span::{Position, Span};

    fn make_bindings(pairs: &[(&str, &str)]) -> CapturedBindings {
        let mut bindings = CapturedBindings::new();
        let file_id = FileId::new("test.php");
        for (name, value) in pairs {
            let span = Span {
                file_id,
                start: Position { offset: 0 },
                end: Position { offset: 0 },
            };
            bindings.insert(name.to_string(), value.to_string(), span);
        }
        bindings
    }

    #[test]
    fn test_simple_template() {
        let bindings = make_bindings(&[("expr", "$x")]);
        let result = Replacer::substitute_template("$expr === null", &bindings);
        assert_eq!(result, "$x === null");
    }

    #[test]
    fn test_multiple_vars() {
        let bindings = make_bindings(&[("var", "$x"), ("default", "'fallback'")]);
        let result = Replacer::substitute_template("$var ?? $default", &bindings);
        assert_eq!(result, "$x ?? 'fallback'");
    }

    #[test]
    fn test_func_call_node() {
        let bindings = make_bindings(&[("haystack", "$str"), ("needle", "'x'")]);
        let node = ReplacementNode {
            node: "FuncCall".to_string(),
            name: Some("str_contains".to_string()),
            args: vec!["$haystack".to_string(), "$needle".to_string()],
            operator: None,
            left: None,
            right: None,
            expr: None,
            wrap: None,
        };
        let result = Replacer::build_node(&node, &bindings);
        assert_eq!(result, Some("str_contains($str, 'x')".to_string()));
    }

    #[test]
    fn test_binary_op_node() {
        let bindings = make_bindings(&[("expr", "$x")]);
        let node = ReplacementNode {
            node: "BinaryOp".to_string(),
            name: None,
            args: vec![],
            operator: Some("===".to_string()),
            left: Some("$expr".to_string()),
            right: Some("null".to_string()),
            expr: None,
            wrap: None,
        };
        let result = Replacer::build_node(&node, &bindings);
        assert_eq!(result, Some("$x === null".to_string()));
    }

    #[test]
    fn test_condition_value_greater() {
        let bindings = make_bindings(&[("len", "5")]);
        assert!(Replacer::evaluate_condition("$len.value > 0", &bindings));
        assert!(!Replacer::evaluate_condition("$len.value > 10", &bindings));
    }

    #[test]
    fn test_condition_exists() {
        let bindings = make_bindings(&[("x", "value")]);
        assert!(Replacer::evaluate_condition("$x.exists", &bindings));
        assert!(!Replacer::evaluate_condition("$y.exists", &bindings));
    }

    #[test]
    fn test_spread_template_substitution() {
        // Test that spread captures like $args are properly substituted
        let bindings = make_bindings(&[("args", "',', $arr")]);
        let result = Replacer::substitute_template("implode($args)", &bindings);
        assert_eq!(result, "implode(',', $arr)");
    }
}
