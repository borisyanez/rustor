//! Fixer presets (PSR-12, Symfony, etc.)
//!
//! Maps PHP-CS-Fixer preset names to their constituent rules.

/// Available presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Psr12,
    PerCs,
    Symfony,
    PhpCsFixer,
}

impl Preset {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "psr12" | "psr-12" | "@psr12" | "@psr-12" => Some(Preset::Psr12),
            "per" | "per-cs" | "percs" | "@per" | "@per-cs" => Some(Preset::PerCs),
            "symfony" | "@symfony" => Some(Preset::Symfony),
            "phpcsfixer" | "php_cs_fixer" | "@phpcsfixer" => Some(Preset::PhpCsFixer),
            _ => None,
        }
    }
}

/// Get the list of rules for a preset
pub fn get_preset_rules(preset_name: &str) -> &'static [&'static str] {
    match preset_name.to_uppercase().replace("-", "").as_str() {
        "PSR12" | "@PSR12" => PSR12_RULES,
        "PER" | "PERCS" | "@PER" | "@PERCS" => PERCS_RULES,
        "SYMFONY" | "@SYMFONY" => SYMFONY_RULES,
        "PHPCSFIXER" | "@PHPCSFIXER" => PHPCSFIXER_RULES,
        _ => &[],
    }
}

/// PSR-12 preset rules
pub const PSR12_RULES: &[&str] = &[
    // Whitespace
    "encoding",
    "full_opening_tag",
    "blank_line_after_opening_tag",
    "no_closing_tag",
    "indentation_type",
    "line_ending",
    "no_trailing_whitespace",
    "no_whitespace_in_blank_line",
    "single_blank_line_at_end_of_file",

    // Casing
    "constant_case",
    "lowercase_keywords",
    "lowercase_static_reference",

    // Namespaces and imports
    "blank_line_after_namespace",
    "no_leading_import_slash",
    "ordered_imports",
    "single_import_per_statement",
    "single_line_after_imports",

    // Class structure
    "braces_position",
    "class_definition",
    "no_blank_lines_after_class_opening",
    "single_class_element_per_statement",
    "single_trait_insert_per_statement",
    "visibility_required",

    // Control structures
    "control_structure_braces",
    "control_structure_continuation_position",
    "elseif",
    "no_alternative_syntax",
    "no_unneeded_braces",
    "single_space_around_construct",
    "switch_case_semicolon_to_colon",
    "switch_case_space",

    // Functions
    "compact_nullable_type_declaration",
    "declare_equal_normalize",
    "function_declaration",
    "method_argument_space",
    "no_spaces_after_function_name",
    "return_type_declaration",

    // Operators and spacing
    "binary_operator_spaces",
    "concat_space",
    "new_with_parentheses",
    "no_space_around_double_colon",
    "object_operator_without_whitespace",
    "ternary_operator_spaces",
    "unary_operator_spaces",

    // Comments
    "no_trailing_whitespace_in_comment",
    "single_line_comment_style",
];

/// PER-CS preset rules (PHP Evolved Recommendations - extends PSR-12)
/// PER-CS 2.0 is the modern evolution of PSR-12
pub const PERCS_RULES: &[&str] = &[
    // All PSR-12 rules
    "encoding",
    "full_opening_tag",
    "blank_line_after_opening_tag",
    "no_closing_tag",
    "indentation_type",
    "line_ending",
    "no_trailing_whitespace",
    "no_whitespace_in_blank_line",
    "single_blank_line_at_end_of_file",
    "constant_case",
    "lowercase_keywords",
    "lowercase_static_reference",
    "blank_line_after_namespace",
    "no_leading_import_slash",
    "ordered_imports",
    "single_import_per_statement",
    "single_line_after_imports",
    "braces_position",
    "class_definition",
    "no_blank_lines_after_class_opening",
    "single_class_element_per_statement",
    "single_trait_insert_per_statement",
    "visibility_required",
    "control_structure_braces",
    "control_structure_continuation_position",
    "elseif",
    "no_alternative_syntax",
    "no_unneeded_braces",
    "switch_case_semicolon_to_colon",
    "switch_case_space",
    "compact_nullable_type_declaration",
    "declare_equal_normalize",
    "function_declaration",
    "method_argument_space",
    "no_spaces_after_function_name",
    "return_type_declaration",
    "binary_operator_spaces",
    "concat_space",
    "new_with_parentheses",
    "no_space_around_double_colon",
    "object_operator_without_whitespace",
    "ternary_operator_spaces",
    "unary_operator_spaces",
    "no_trailing_whitespace_in_comment",
    "single_line_comment_style",

    // PER-CS 2.0 additions (modern PHP features)
    "native_function_casing",
    "magic_method_casing",
    "magic_constant_casing",
    "ordered_class_elements",
    "method_chaining_indentation",
];

/// Symfony preset rules (extends PSR-12)
pub const SYMFONY_RULES: &[&str] = &[
    // All PSR-12 rules
    "encoding",
    "full_opening_tag",
    "blank_line_after_opening_tag",
    "no_closing_tag",
    "indentation_type",
    "line_ending",
    "no_trailing_whitespace",
    "no_whitespace_in_blank_line",
    "single_blank_line_at_end_of_file",
    "constant_case",
    "lowercase_keywords",
    "lowercase_static_reference",
    "blank_line_after_namespace",
    "no_leading_import_slash",
    "ordered_imports",
    "single_import_per_statement",
    "single_line_after_imports",
    "braces_position",
    "class_definition",
    "no_blank_lines_after_class_opening",
    "single_class_element_per_statement",
    "single_trait_insert_per_statement",
    "visibility_required",
    "control_structure_braces",
    "control_structure_continuation_position",
    "elseif",
    "no_alternative_syntax",
    "no_unneeded_braces",
    "switch_case_semicolon_to_colon",
    "switch_case_space",
    "compact_nullable_type_declaration",
    "declare_equal_normalize",
    "function_declaration",
    "method_argument_space",
    "no_spaces_after_function_name",
    "return_type_declaration",
    "binary_operator_spaces",
    "concat_space",
    "new_with_parentheses",
    "no_space_around_double_colon",
    "object_operator_without_whitespace",
    "ternary_operator_spaces",
    "unary_operator_spaces",
    "no_trailing_whitespace_in_comment",
    "single_line_comment_style",

    // Additional Symfony rules
    "array_syntax",
    "backtick_to_shell_exec",
    "blank_line_before_statement",
    "cast_spaces",
    "class_attributes_separation",
    "clean_namespace",
    "echo_tag_syntax",
    "empty_loop_body",
    "empty_loop_condition",
    "fully_qualified_strict_types",
    "function_typehint_space",
    "global_namespace_import",
    "include",
    "increment_style",
    "integer_literal_case",
    "lambda_not_used_import",
    "linebreak_after_opening_tag",
    "magic_constant_casing",
    "magic_method_casing",
    "method_chaining_indentation",
    "native_function_casing",
    "native_type_declaration_casing",
    "no_alias_language_construct_call",
    "no_blank_lines_after_phpdoc",
    "no_empty_comment",
    "no_empty_phpdoc",
    "no_empty_statement",
    "no_extra_blank_lines",
    "no_mixed_echo_print",
    "no_multiline_whitespace_around_double_arrow",
    "no_null_property_initialization",
    "no_short_bool_cast",
    "no_singleline_whitespace_before_semicolons",
    "no_spaces_around_offset",
    "no_superfluous_elseif",
    "no_superfluous_phpdoc_tags",
    "no_unneeded_control_parentheses",
    "no_unneeded_import_alias",
    "no_unset_cast",
    "no_unused_imports",
    "no_useless_concat_operator",
    "no_useless_else",
    "no_useless_nullsafe_operator",
    "no_useless_return",
    "no_whitespace_before_comma_in_array",
    "normalize_index_brace",
    "nullable_type_declaration",
    "nullable_type_declaration_for_default_null_value",
    "operator_linebreak",
    "ordered_types",
    "php_unit_fqcn_annotation",
    "php_unit_method_casing",
    "phpdoc_align",
    "phpdoc_annotation_without_dot",
    "phpdoc_indent",
    "phpdoc_inline_tag_normalizer",
    "phpdoc_no_access",
    "phpdoc_no_alias_tag",
    "phpdoc_no_package",
    "phpdoc_no_useless_inheritdoc",
    "phpdoc_order",
    "phpdoc_return_self_reference",
    "phpdoc_scalar",
    "phpdoc_separation",
    "phpdoc_single_line_var_spacing",
    "phpdoc_summary",
    "phpdoc_tag_type",
    "phpdoc_to_comment",
    "phpdoc_trim",
    "phpdoc_trim_consecutive_blank_line_separation",
    "phpdoc_types",
    "phpdoc_types_order",
    "phpdoc_var_without_name",
    "semicolon_after_instruction",
    "simple_to_complex_string_variable",
    "simplified_if_return",
    "simplified_null_return",
    "single_line_comment_spacing",
    "single_line_empty_body",
    "single_line_throw",
    "single_quote",
    "single_space_around_construct",
    "space_after_semicolon",
    "standardize_increment",
    "standardize_not_equals",
    "switch_continue_to_break",
    "trailing_comma_in_multiline",
    "trim_array_spaces",
    "type_declaration_spaces",
    "types_spaces",
    "whitespace_after_comma_in_array",
    "yoda_style",
];

/// PHP-CS-Fixer preset rules (extends Symfony)
pub const PHPCSFIXER_RULES: &[&str] = &[
    // Include all Symfony rules plus additional ones
    // This is a subset - the full list is very long
    "encoding",
    "full_opening_tag",
    "blank_line_after_opening_tag",
    "no_closing_tag",
    "indentation_type",
    "line_ending",
    "no_trailing_whitespace",
    "single_blank_line_at_end_of_file",
    "array_syntax",
    "braces_position",
    "class_definition",
    "concat_space",
    "method_argument_space",
    "ordered_imports",
    "single_quote",
    "yoda_style",
    // Additional strictness
    "strict_comparison",
    "strict_param",
    "declare_strict_types",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_from_str() {
        assert_eq!(Preset::from_str("psr12"), Some(Preset::Psr12));
        assert_eq!(Preset::from_str("PSR-12"), Some(Preset::Psr12));
        assert_eq!(Preset::from_str("@PSR12"), Some(Preset::Psr12));
        assert_eq!(Preset::from_str("per"), Some(Preset::PerCs));
        assert_eq!(Preset::from_str("per-cs"), Some(Preset::PerCs));
        assert_eq!(Preset::from_str("@per-cs"), Some(Preset::PerCs));
        assert_eq!(Preset::from_str("symfony"), Some(Preset::Symfony));
        assert_eq!(Preset::from_str("unknown"), None);
    }

    #[test]
    fn test_get_preset_rules() {
        let psr12 = get_preset_rules("PSR12");
        assert!(psr12.contains(&"indentation_type"));
        assert!(psr12.contains(&"line_ending"));
        assert!(psr12.contains(&"no_trailing_whitespace"));

        let percs = get_preset_rules("PERCS");
        assert!(percs.contains(&"native_function_casing"));
        assert!(percs.contains(&"magic_method_casing"));
        assert!(percs.contains(&"ordered_class_elements"));
    }

    #[test]
    fn test_psr12_has_core_rules() {
        assert!(PSR12_RULES.contains(&"encoding"));
        assert!(PSR12_RULES.contains(&"full_opening_tag"));
        assert!(PSR12_RULES.contains(&"lowercase_keywords"));
        assert!(PSR12_RULES.contains(&"visibility_required"));
    }
}
