//! rustor-rector-import: Import Rector PHP rules into rustor
//!
//! This crate provides tools to:
//! 1. Parse Rector PHP rule files
//! 2. Extract rule patterns and metadata
//! 3. Generate equivalent rustor Rust rules

pub mod ast_mapper;
pub mod codegen;
pub mod pattern_detector;
pub mod php_parser;
pub mod report;
pub mod rule_extractor;
pub mod templates;

use serde::{Deserialize, Serialize};

/// Metadata extracted from a Rector rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectorRule {
    /// Rule class name (e.g., "IsNullRector")
    pub name: String,

    /// Category/directory (e.g., "CodeQuality", "Php80")
    pub category: String,

    /// Rule description from getRuleDefinition()
    pub description: String,

    /// AST node types the rule handles (e.g., ["FuncCall", "Identical"])
    pub node_types: Vec<String>,

    /// Minimum PHP version required (e.g., "8.0")
    pub min_php_version: Option<String>,

    /// Code sample before transformation
    pub before_code: String,

    /// Code sample after transformation
    pub after_code: String,

    /// Detected rule pattern for code generation
    pub pattern: RulePattern,

    /// Whether the rule is configurable
    pub is_configurable: bool,

    /// Source file path
    pub source_file: String,
}

/// Recognized rule patterns for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RulePattern {
    /// Simple function rename: join() → implode()
    FunctionRename {
        from: String,
        to: String,
    },

    /// Function call to operator: pow($x, 2) → $x ** 2
    FunctionToOperator {
        func: String,
        operator: String,
        arg_positions: Vec<usize>,
    },

    /// Function to comparison: is_null($x) → $x === null
    FunctionToComparison {
        func: String,
        operator: String,
        compare_value: String,
    },

    /// Function to method call: sizeof($x) → count($x)
    FunctionAlias {
        from: String,
        to: String,
    },

    /// Type cast conversion: strval($x) → (string) $x
    FunctionToCast {
        func: String,
        cast_type: String,
    },

    /// Ternary to coalesce: isset($x) ? $x : $d → $x ?? $d
    TernaryToCoalesce {
        condition_func: String,
    },

    /// Array syntax: array() → []
    ArraySyntaxModern,

    /// Closure to arrow function
    ClosureToArrow,

    /// Function to ::class constant: get_class($obj) → $obj::class
    FunctionToClassConstant {
        func: String,
    },

    /// Function to instanceof: is_a($obj, Class::class) → $obj instanceof Class
    FunctionToInstanceof {
        func: String,
    },

    /// Unwrap single-arg function that returns its arg: sprintf($x) → $x
    UnwrapSingleArgFunction {
        func: String,
    },

    /// Remove first argument: implode(',', $arr) → implode($arr) (PHP 7.4+)
    FunctionRemoveFirstArg {
        func: String,
    },

    /// Function without args to another: mktime() → time()
    FunctionNoArgsToFunction {
        from: String,
        to: String,
    },

    /// Nullsafe method call: $x ? $x->y() : null → $x?->y()
    NullsafeMethodCall,

    /// First-class callable syntax: Closure::fromCallable([$this, 'method']) → $this->method(...)
    FirstClassCallable,

    /// Complex pattern requiring manual implementation
    Complex {
        hints: Vec<String>,
        refactor_body: String,
    },

    /// Unknown/unrecognized pattern
    Unknown,
}

impl RulePattern {
    /// Check if this pattern can be auto-generated
    pub fn is_auto_generatable(&self) -> bool {
        !matches!(self, RulePattern::Complex { .. } | RulePattern::Unknown)
    }

    /// Get pattern type name for reporting
    pub fn type_name(&self) -> &'static str {
        match self {
            RulePattern::FunctionRename { .. } => "FunctionRename",
            RulePattern::FunctionToOperator { .. } => "FunctionToOperator",
            RulePattern::FunctionToComparison { .. } => "FunctionToComparison",
            RulePattern::FunctionAlias { .. } => "FunctionAlias",
            RulePattern::FunctionToCast { .. } => "FunctionToCast",
            RulePattern::TernaryToCoalesce { .. } => "TernaryToCoalesce",
            RulePattern::ArraySyntaxModern => "ArraySyntaxModern",
            RulePattern::ClosureToArrow => "ClosureToArrow",
            RulePattern::FunctionToClassConstant { .. } => "FunctionToClassConstant",
            RulePattern::FunctionToInstanceof { .. } => "FunctionToInstanceof",
            RulePattern::UnwrapSingleArgFunction { .. } => "UnwrapSingleArgFunction",
            RulePattern::FunctionRemoveFirstArg { .. } => "FunctionRemoveFirstArg",
            RulePattern::FunctionNoArgsToFunction { .. } => "FunctionNoArgsToFunction",
            RulePattern::NullsafeMethodCall => "NullsafeMethodCall",
            RulePattern::FirstClassCallable => "FirstClassCallable",
            RulePattern::Complex { .. } => "Complex",
            RulePattern::Unknown => "Unknown",
        }
    }
}

/// Result of importing rules
#[derive(Debug, Default)]
pub struct ImportResult {
    /// Successfully parsed rules
    pub rules: Vec<RectorRule>,

    /// Rules that couldn't be parsed
    pub failed: Vec<(String, String)>, // (file, error)

    /// Warnings during parsing
    pub warnings: Vec<String>,
}

impl ImportResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Count of auto-generatable rules
    pub fn auto_generatable_count(&self) -> usize {
        self.rules.iter().filter(|r| r.pattern.is_auto_generatable()).count()
    }

    /// Count by pattern type
    pub fn count_by_pattern(&self) -> std::collections::HashMap<&'static str, usize> {
        let mut counts = std::collections::HashMap::new();
        for rule in &self.rules {
            *counts.entry(rule.pattern.type_name()).or_insert(0) += 1;
        }
        counts
    }
}
