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

pub mod array_first_last;
pub mod array_key_first_last;
pub mod array_push;
pub mod arrow_functions;
pub mod assign_coalesce;
pub mod array_syntax;
pub mod class_constructor;
pub mod class_on_object;
pub mod constructor_promotion;
pub mod empty_coalesce;
pub mod explicit_nullable_param;
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
pub mod rename_attribute;
pub mod rename_class;
pub mod rename_class_const;
pub mod rename_constant;
pub mod rename_function;
pub mod rename_static_method;
pub mod rename_string;
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
pub mod array_key_exists_to_coalesce;
pub mod simplify_empty_array_check;
pub mod remove_concat_autocast;
pub mod common_not_equal;
pub mod join_string_concat;
pub mod sensitive_define;
pub mod flip_negated_ternary_instanceof;
pub mod foreach_to_array_all;
pub mod foreach_to_array_any;
pub mod foreach_to_array_find;
pub mod foreach_to_array_find_key;
pub mod simplify_tautology_ternary;
pub mod dirname_file_to_dir;
pub mod ternary_implode_to_implode;
pub mod dirname_dir_concat;
pub mod strict_array_search;
pub mod simplify_array_search;
pub mod version_compare_to_constant;
pub mod strlen_to_empty_string;
pub mod mktime_to_time;
pub mod multi_dirname;
pub mod stringify_define;
pub mod array_merge_simple;
pub mod random_function;
pub mod remove_get_class_no_args;
pub mod rounding_mode_enum;
pub mod dynamic_class_const_fetch;
pub mod replace_http_server_vars;
pub mod class_constant_to_self_class;
pub mod remove_reference_from_call;
pub mod ternary_to_elvis;
pub mod remove_zero_break_continue;
pub mod get_called_class_to_static;
pub mod post_to_pre_increment;
pub mod separate_multi_use_imports;
pub mod simplify_quote_escape;
pub mod split_double_assign;
pub mod split_grouped_class_constants;
pub mod split_grouped_properties;
pub mod func_call_to_const_fetch;
pub mod func_call_to_new;
pub mod const_fetch_to_class_const_fetch;
pub mod func_call_to_static_call;
pub mod string_to_class_constant;
pub mod new_to_static_call;

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
pub use array_first_last::check_array_first_last;
pub use array_key_first_last::check_array_key_first_last;
pub use array_push::check_array_push;
pub use arrow_functions::check_arrow_functions;
pub use assign_coalesce::check_assign_coalesce;
pub use array_syntax::check_array_syntax;
pub use class_constructor::check_class_constructor;
pub use constructor_promotion::check_constructor_promotion;
pub use empty_coalesce::check_empty_coalesce;
pub use explicit_nullable_param::check_explicit_nullable_param;
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
pub use array_key_exists_to_coalesce::check_array_key_exists_to_coalesce;
pub use simplify_empty_array_check::check_simplify_empty_array_check;
pub use remove_concat_autocast::check_remove_concat_autocast;
pub use common_not_equal::check_common_not_equal;
pub use join_string_concat::check_join_string_concat;
pub use sensitive_define::check_sensitive_define;
pub use flip_negated_ternary_instanceof::check_flip_negated_ternary_instanceof;
pub use foreach_to_array_all::check_foreach_to_array_all;
pub use foreach_to_array_any::check_foreach_to_array_any;
pub use foreach_to_array_find::check_foreach_to_array_find;
pub use foreach_to_array_find_key::check_foreach_to_array_find_key;
pub use simplify_tautology_ternary::check_simplify_tautology_ternary;
pub use dirname_file_to_dir::check_dirname_file_to_dir;
pub use ternary_implode_to_implode::check_ternary_implode_to_implode;
pub use dirname_dir_concat::check_dirname_dir_concat;
pub use strict_array_search::check_strict_array_search;
pub use simplify_array_search::check_simplify_array_search;
pub use version_compare_to_constant::check_version_compare_to_constant;
pub use strlen_to_empty_string::check_strlen_to_empty_string;
pub use mktime_to_time::check_mktime_to_time;
pub use multi_dirname::check_multi_dirname;
pub use stringify_define::check_stringify_define;
pub use array_merge_simple::check_array_merge_simple;
pub use random_function::check_random_function;
pub use remove_get_class_no_args::check_remove_get_class_no_args;
pub use rounding_mode_enum::check_rounding_mode_enum;
pub use dynamic_class_const_fetch::check_dynamic_class_const_fetch;
pub use replace_http_server_vars::check_replace_http_server_vars;
pub use class_constant_to_self_class::check_class_constant_to_self_class;
pub use remove_reference_from_call::check_remove_reference_from_call;
pub use ternary_to_elvis::check_ternary_to_elvis;
pub use remove_zero_break_continue::check_remove_zero_break_continue;
pub use get_called_class_to_static::check_get_called_class_to_static;
