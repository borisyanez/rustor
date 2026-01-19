//! YAML rule schema definitions
//!
//! Defines the structure of YAML-based refactoring rules using serde
//! for deserialization from YAML format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete YAML-defined refactoring rule
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct YamlRule {
    /// Unique rule identifier (e.g., "is_null_to_comparison")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Rule category (code_quality, dead_code, php70, php80, etc.)
    #[serde(default)]
    pub category: String,

    /// Minimum PHP version required (e.g., "7.0", "8.0")
    #[serde(default)]
    pub min_php: Option<String>,

    /// Maximum PHP version (optional upper bound)
    #[serde(default)]
    pub max_php: Option<String>,

    /// Pattern to match in the AST
    #[serde(rename = "match")]
    pub match_pattern: MatchPattern,

    /// Replacement to produce
    pub replace: Replacement,

    /// Optional conditions that must be true for the rule to apply
    #[serde(default)]
    pub when: Vec<RuleCondition>,

    /// Test cases (required for validation)
    #[serde(default)]
    pub tests: Vec<TestCase>,

    /// Whether this rule is configurable
    #[serde(default)]
    pub configurable: bool,

    /// Configuration schema for configurable rules
    #[serde(default)]
    pub config: Option<ConfigSchema>,
}

/// Pattern to match in the PHP AST
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MatchPattern {
    /// Match a single node type
    Node(NodePattern),

    /// Match any of multiple patterns (OR)
    Any { any: Vec<MatchPattern> },

    /// Match all patterns (AND)
    All { all: Vec<MatchPattern> },
}

/// Pattern for matching a specific AST node
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodePattern {
    /// AST node type to match (e.g., FuncCall, MethodCall, BinaryOp)
    pub node: String,

    /// Function/method name to match (for call nodes)
    #[serde(default)]
    pub name: Option<StringOrCapture>,

    /// Class name (for static calls)
    #[serde(default)]
    pub class: Option<StringOrCapture>,

    /// Method name (for method calls)
    #[serde(default)]
    pub method: Option<StringOrCapture>,

    /// Object being called on (for method calls)
    #[serde(default)]
    pub object: Option<Box<CaptureOrPattern>>,

    /// Arguments to match
    #[serde(default)]
    pub args: Vec<ArgPattern>,

    /// Binary operator (for BinaryOp nodes)
    #[serde(default)]
    pub operator: Option<String>,

    /// Left side of binary op
    #[serde(default)]
    pub left: Option<Box<CaptureOrPattern>>,

    /// Right side of binary op
    #[serde(default)]
    pub right: Option<Box<CaptureOrPattern>>,

    /// Condition (for ternary/if)
    #[serde(default)]
    pub condition: Option<Box<CaptureOrPattern>>,

    /// Then branch (for ternary)
    #[serde(default)]
    pub then: Option<Box<CaptureOrPattern>>,

    /// Else branch (for ternary)
    #[serde(default, rename = "else")]
    pub else_branch: Option<Box<CaptureOrPattern>>,

    /// Array syntax type (long/short)
    #[serde(default)]
    pub syntax: Option<String>,

    /// Array items
    #[serde(default)]
    pub items: Option<Vec<ArgPattern>>,
}

/// Either a literal string or a capture variable
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringOrCapture {
    /// Literal string value
    Literal(String),
    /// Capture variable (e.g., "$name")
    Capture { capture: String },
}

impl StringOrCapture {
    /// Check if this is a capture variable
    pub fn is_capture(&self) -> bool {
        matches!(self, StringOrCapture::Capture { .. })
    }

    /// Get the literal value if this is a literal
    pub fn as_literal(&self) -> Option<&str> {
        match self {
            StringOrCapture::Literal(s) => Some(s),
            StringOrCapture::Capture { .. } => None,
        }
    }

    /// Get the capture name if this is a capture
    pub fn as_capture(&self) -> Option<&str> {
        match self {
            StringOrCapture::Literal(_) => None,
            StringOrCapture::Capture { capture } => Some(capture),
        }
    }
}

/// Either a capture variable or a nested pattern
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CaptureOrPattern {
    /// Simple capture variable (e.g., "$expr")
    Capture(String),

    /// Capture with capture keyword
    CaptureExplicit { capture: String },

    /// Reference to same captured value
    SameAs { same_as: String },

    /// Nested node pattern
    Pattern(NodePattern),
}

impl CaptureOrPattern {
    /// Check if this is a capture
    pub fn is_capture(&self) -> bool {
        matches!(
            self,
            CaptureOrPattern::Capture(_) | CaptureOrPattern::CaptureExplicit { .. }
        )
    }

    /// Get the capture name
    pub fn capture_name(&self) -> Option<&str> {
        match self {
            CaptureOrPattern::Capture(s) => {
                // Strip leading $ if present
                Some(s.strip_prefix('$').unwrap_or(s))
            }
            CaptureOrPattern::CaptureExplicit { capture } => {
                Some(capture.strip_prefix('$').unwrap_or(capture))
            }
            CaptureOrPattern::SameAs { same_as } => {
                Some(same_as.strip_prefix('$').unwrap_or(same_as))
            }
            CaptureOrPattern::Pattern(_) => None,
        }
    }
}

/// Pattern for matching function arguments
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgPattern {
    /// Capture an argument
    Capture { capture: String },

    /// Match a literal value
    Literal { literal: ArgValue },

    /// Spread capture for remaining args
    Spread { capture: String },

    /// No more arguments allowed after this
    NoMore { no_more: bool },

    /// Optional argument
    Optional { optional: bool, capture: String },
}

/// Literal argument value
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Null,
}

/// Replacement specification
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Replacement {
    /// Simple string template with variable substitution
    Template(String),

    /// Structured node replacement
    Node(ReplacementNode),

    /// Conditional replacement
    Conditional(ConditionalReplacement),

    /// Multiple replacements (node expansion)
    Multiple { multiple: Vec<String> },

    /// Remove the matched node
    Remove,
}

impl Replacement {
    /// Check if this is a remove replacement
    pub fn is_remove(&self) -> bool {
        matches!(self, Replacement::Remove)
    }
}

/// Structured replacement node
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReplacementNode {
    /// Node type to create
    pub node: String,

    /// Function name (for FuncCall)
    #[serde(default)]
    pub name: Option<String>,

    /// Arguments
    #[serde(default)]
    pub args: Vec<String>,

    /// Binary operator
    #[serde(default)]
    pub operator: Option<String>,

    /// Left operand
    #[serde(default)]
    pub left: Option<String>,

    /// Right operand
    #[serde(default)]
    pub right: Option<String>,

    /// Expression to wrap (for wrap operations)
    #[serde(default)]
    pub expr: Option<String>,

    /// Wrap specification
    #[serde(default)]
    pub wrap: Option<Box<ReplacementNode>>,
}

/// Conditional replacement
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConditionalReplacement {
    /// Condition to evaluate
    #[serde(rename = "if")]
    pub condition: ConditionExpr,

    /// Replacement if condition is true
    #[serde(rename = "then")]
    pub then_replace: Box<Replacement>,

    /// Replacement if condition is false
    #[serde(rename = "else")]
    pub else_replace: Box<Replacement>,
}

/// Condition expression for conditional replacements
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConditionExpr {
    /// Condition string to evaluate
    pub condition: String,
}

/// Condition that must be true for rule to apply
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RuleCondition {
    /// Type condition (e.g., $expr.type: string)
    Type { var: String, type_is: String },

    /// Value condition (e.g., $count.value: "> 0")
    Value { var: String, value: String },

    /// Context condition (e.g., in_class: true)
    Context(ContextCondition),
}

/// Context-based conditions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextCondition {
    /// Must be inside a class
    #[serde(default)]
    pub in_class: Option<bool>,

    /// Must be inside a function
    #[serde(default)]
    pub in_function: Option<bool>,

    /// Must have specific parent node
    #[serde(default)]
    pub has_parent: Option<String>,
}

/// Test case for rule validation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestCase {
    /// Input PHP code
    pub input: String,

    /// Expected output (if transformation should occur)
    #[serde(default)]
    pub output: Option<String>,

    /// Skip this test case
    #[serde(default)]
    pub skip: bool,

    /// Configuration for this test case
    #[serde(default)]
    pub config: Option<HashMap<String, serde_yaml::Value>>,
}

/// Configuration schema for configurable rules
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigSchema {
    /// Configuration options
    #[serde(flatten)]
    pub options: HashMap<String, ConfigOption>,
}

/// A configuration option definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigOption {
    /// Option type
    #[serde(rename = "type")]
    pub option_type: String,

    /// Description
    #[serde(default)]
    pub description: Option<String>,

    /// Example value
    #[serde(default)]
    pub example: Option<serde_yaml::Value>,
}

impl YamlRule {
    /// Validate the rule structure
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Rule name is required".to_string());
        }
        if self.description.is_empty() {
            return Err("Rule description is required".to_string());
        }
        // Tests are optional but recommended
        Ok(())
    }

    /// Get minimum PHP version as a comparable tuple
    pub fn min_php_version(&self) -> Option<(u8, u8)> {
        self.min_php.as_ref().and_then(|v| {
            let parts: Vec<&str> = v.split('.').collect();
            if parts.len() >= 2 {
                let major = parts[0].parse().ok()?;
                let minor = parts[1].parse().ok()?;
                Some((major, minor))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let yaml = r#"
name: is_null_to_comparison
description: Replace is_null($x) with $x === null
category: code_quality
min_php: "7.0"

match:
  node: FuncCall
  name: is_null
  args:
    - capture: $expr

replace: "$expr === null"

tests:
  - input: "is_null($x)"
    output: "$x === null"
"#;
        let rule: YamlRule = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(rule.name, "is_null_to_comparison");
        assert_eq!(rule.min_php, Some("7.0".to_string()));
        assert_eq!(rule.tests.len(), 1);
    }

    #[test]
    fn test_parse_complex_match() {
        let yaml = r#"
name: ternary_to_coalesce
description: "Convert isset($x) ? $x : $default to $x ?? $default"
min_php: "7.0"

match:
  node: Ternary
  condition:
    capture: $cond
  then:
    capture: $then
  else:
    capture: $default

replace: "$then ?? $default"

tests:
  - input: "isset($x) ? $x : 'default'"
    output: "$x ?? 'default'"
"#;
        let rule: YamlRule = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(rule.name, "ternary_to_coalesce");

        // Check the match pattern structure
        if let MatchPattern::Node(node) = &rule.match_pattern {
            assert_eq!(node.node, "Ternary");
            assert!(node.condition.is_some());
        } else {
            panic!("Expected Node pattern");
        }
    }

    #[test]
    fn test_parse_any_match() {
        let yaml = r#"
name: strpos_to_str_contains
description: Convert strpos() !== false to str_contains()
min_php: "8.0"

match:
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
        node: LiteralFalse

replace: "str_contains($haystack, $needle)"

tests:
  - input: "strpos($str, 'x') !== false"
    output: "str_contains($str, 'x')"
"#;
        let rule: YamlRule = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(rule.name, "strpos_to_str_contains");

        // Check it's an Any pattern
        if let MatchPattern::Any { any } = &rule.match_pattern {
            assert_eq!(any.len(), 2);
        } else {
            panic!("Expected Any pattern");
        }
    }
}
