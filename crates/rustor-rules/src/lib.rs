//! rustor-rules: Refactoring rule implementations
//!
//! Available rules:
//! - array_push: Convert array_push($arr, $val) to $arr[] = $val
//! - array_syntax: Convert array() to [] (short array syntax)
//! - empty_coalesce: Convert empty($x) ? $default : $x to $x ?: $default
//! - is_null: Convert is_null($x) to $x === null
//! - isset_coalesce: Convert isset($x) ? $x : $default to $x ?? $default
//! - join_to_implode: Convert join() to implode()
//! - list_short_syntax: Convert list($a, $b) to [$a, $b]
//! - pow_to_operator: Convert pow($x, $n) to $x ** $n
//! - sizeof: Convert sizeof($x) to count($x)
//! - type_cast: Convert strval/intval/floatval to cast syntax

pub mod registry;

pub mod array_push;
pub mod array_syntax;
pub mod empty_coalesce;
pub mod is_null;
pub mod isset_coalesce;
pub mod join_to_implode;
pub mod list_short_syntax;
pub mod pow_to_operator;
pub mod sizeof;
pub mod type_cast;

// Re-export the Rule trait and registry
pub use registry::{Rule, RuleRegistry};

// Re-export check functions (for backwards compatibility)
pub use array_push::check_array_push;
pub use array_syntax::check_array_syntax;
pub use empty_coalesce::check_empty_coalesce;
pub use is_null::check_is_null;
pub use isset_coalesce::check_isset_coalesce;
pub use join_to_implode::check_join_to_implode;
pub use list_short_syntax::check_list_short_syntax;
pub use pow_to_operator::check_pow_to_operator;
pub use sizeof::check_sizeof;
pub use type_cast::check_type_cast;
