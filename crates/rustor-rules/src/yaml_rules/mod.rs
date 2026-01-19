//! YAML-based rule engine for defining refactoring rules declaratively
//!
//! This module provides a complete system for expressing PHP refactoring rules
//! in YAML format, enabling:
//! - Easy rule authoring by non-Rust developers
//! - Runtime interpretation without compilation
//! - Inline test cases for validation
//! - Full coverage of Rector rule patterns
//!
//! # Example YAML Rule
//!
//! ```yaml
//! name: is_null_to_comparison
//! description: Replace is_null($x) with $x === null
//! category: code_quality
//! min_php: 7.0
//!
//! match:
//!   node: FuncCall
//!   name: is_null
//!   args:
//!     - capture: $expr
//!
//! replace: "$expr === null"
//!
//! tests:
//!   - input: "is_null($x)"
//!     output: "$x === null"
//! ```

pub mod schema;
pub mod matcher;
pub mod replacer;
pub mod interpreter;
pub mod loader;

pub use schema::{YamlRule, MatchPattern, Replacement, TestCase, RuleCondition};
pub use matcher::{PatternMatcher, CapturedBindings};
pub use replacer::Replacer;
pub use interpreter::YamlRuleInterpreter;
pub use loader::{load_rules_from_file, load_rules_from_dir, load_rules_from_string};
