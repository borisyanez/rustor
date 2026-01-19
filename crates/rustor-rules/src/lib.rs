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
//!
//! # YAML Rules
//!
//! Rules can also be defined in YAML format for easy authoring:
//!
//! ```yaml
//! name: is_null_to_comparison
//! description: Replace is_null($x) with $x === null
//! match:
//!   node: FuncCall
//!   name: is_null
//!   args:
//!     - capture: $expr
//! replace: "$expr === null"
//! tests:
//!   - input: "is_null($x)"
//!     output: "$x === null"
//! ```

pub mod registry;
pub mod yaml_rules;

pub mod imported;

pub mod array_key_first_last;
pub mod array_push;
pub mod arrow_functions;
pub mod assign_coalesce;
pub mod array_syntax;
pub mod class_constructor;
pub mod class_on_object;
pub mod constructor_promotion;
pub mod empty_coalesce;
pub mod filter_var_to_addslashes;
pub mod first_class_callables;
pub mod get_class_this;
pub mod get_debug_type;
pub mod hebrevc_to_nl2br_hebrev;
pub mod implode_order;
pub mod is_null;
pub mod isset_coalesce;
pub mod join_to_implode;
pub mod list_short_syntax;
pub mod match_expression;
pub mod null_safe_operator;
pub mod override_attribute;
pub mod pow_to_operator;
pub mod readonly_properties;
pub mod redundant_type_check;
pub mod rename_class;
pub mod rename_function;
pub mod restore_include_path;
pub mod single_in_array_to_compare;
pub mod sizeof;
pub mod switch_negated_ternary;
pub mod sprintf_positional;
pub mod string_contains;
pub mod string_starts_ends;
pub mod type_cast;
pub mod is_countable;
pub mod simplify_strpos_lower;
pub mod unnecessary_ternary;
pub mod unwrap_sprintf;
pub mod utf8_decode_encode;
pub mod settype_to_cast;
pub mod simplify_in_array_values;
pub mod double_negation_to_cast;
pub mod simplify_func_get_args_count;

// Re-export the Rule trait, registry, and metadata types
pub use registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Preset,
    Rule, RuleConfigs, RuleInfo, RuleRegistry,
};

// Re-export yaml_rules types
pub use yaml_rules::{
    YamlRule, YamlRuleInterpreter, MatchPattern, Replacement, TestCase,
    load_rules_from_file, load_rules_from_dir, load_rules_from_string,
};

// Re-export check functions (for backwards compatibility)
pub use array_key_first_last::check_array_key_first_last;
pub use array_push::check_array_push;
pub use arrow_functions::check_arrow_functions;
pub use assign_coalesce::check_assign_coalesce;
pub use array_syntax::check_array_syntax;
pub use class_constructor::check_class_constructor;
pub use constructor_promotion::check_constructor_promotion;
pub use empty_coalesce::check_empty_coalesce;
pub use first_class_callables::check_first_class_callables;
pub use class_on_object::check_class_on_object;
pub use filter_var_to_addslashes::check_filter_var_to_addslashes;
pub use get_class_this::check_get_class_this;
pub use get_debug_type::check_get_debug_type;
pub use hebrevc_to_nl2br_hebrev::check_hebrevc_to_nl2br_hebrev;
pub use implode_order::check_implode_order;
pub use is_null::check_is_null;
pub use isset_coalesce::check_isset_coalesce;
pub use join_to_implode::check_join_to_implode;
pub use list_short_syntax::check_list_short_syntax;
pub use match_expression::check_match_expression;
pub use null_safe_operator::check_null_safe_operator;
pub use override_attribute::check_override_attribute;
pub use pow_to_operator::check_pow_to_operator;
pub use readonly_properties::check_readonly_properties;
pub use restore_include_path::check_restore_include_path;
pub use sizeof::check_sizeof;
pub use sprintf_positional::check_sprintf_positional;
pub use string_contains::check_string_contains;
pub use string_starts_ends::check_string_starts_ends;
pub use type_cast::check_type_cast;
pub use is_countable::check_is_countable;
pub use simplify_strpos_lower::check_simplify_strpos_lower;
pub use unnecessary_ternary::check_unnecessary_ternary;
pub use unwrap_sprintf::check_unwrap_sprintf;
pub use utf8_decode_encode::check_utf8_decode_encode;
pub use settype_to_cast::check_settype_to_cast;
pub use simplify_in_array_values::check_simplify_in_array_values;
pub use double_negation_to_cast::check_double_negation_to_cast;
pub use simplify_func_get_args_count::check_simplify_func_get_args_count;
