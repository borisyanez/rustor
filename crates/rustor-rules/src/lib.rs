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

pub mod array_key_first_last;
pub mod array_push;
pub mod assign_coalesce;
pub mod array_syntax;
pub mod class_constructor;
pub mod constructor_promotion;
pub mod empty_coalesce;
pub mod first_class_callables;
pub mod is_null;
pub mod isset_coalesce;
pub mod join_to_implode;
pub mod list_short_syntax;
pub mod match_expression;
pub mod null_safe_operator;
pub mod pow_to_operator;
pub mod readonly_properties;
pub mod sizeof;
pub mod sprintf_positional;
pub mod string_contains;
pub mod string_starts_ends;
pub mod type_cast;

// Re-export the Rule trait, registry, and metadata types
pub use registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Preset,
    Rule, RuleConfigs, RuleInfo, RuleRegistry,
};

// Re-export check functions (for backwards compatibility)
pub use array_key_first_last::check_array_key_first_last;
pub use array_push::check_array_push;
pub use assign_coalesce::check_assign_coalesce;
pub use array_syntax::check_array_syntax;
pub use class_constructor::check_class_constructor;
pub use constructor_promotion::check_constructor_promotion;
pub use empty_coalesce::check_empty_coalesce;
pub use first_class_callables::check_first_class_callables;
pub use is_null::check_is_null;
pub use isset_coalesce::check_isset_coalesce;
pub use join_to_implode::check_join_to_implode;
pub use list_short_syntax::check_list_short_syntax;
pub use match_expression::check_match_expression;
pub use null_safe_operator::check_null_safe_operator;
pub use pow_to_operator::check_pow_to_operator;
pub use readonly_properties::check_readonly_properties;
pub use sizeof::check_sizeof;
pub use sprintf_positional::check_sprintf_positional;
pub use string_contains::check_string_contains;
pub use string_starts_ends::check_string_starts_ends;
pub use type_cast::check_type_cast;
