//! Rule: rename_function (Level 6 - Configurable)
//!
//! Renames function calls based on a configurable mapping.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.rename_function]
//! mappings = { "old_name" = "new_name", "legacy_func" = "modern_func" }
//! ```
//!
//! Example transformation:
//! ```php
//! // Before (with mapping: old_name -> new_name)
//! $result = old_name($arg1, $arg2);
//!
//! // After
//! $result = new_name($arg1, $arg2);
//! ```
//!
//! This is a Level 6 rule because:
//! - Requires runtime configuration (the rename mapping)
//! - Behavior is entirely determined by user-provided config
//! - No hardcoded patterns - fully generic

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// Configuration for the rename_function rule
#[derive(Debug, Clone, Default)]
pub struct RenameFunctionConfig {
    /// Map of old function names to new function names
    /// Keys and values are case-insensitive for matching but preserve case in output
    pub mappings: HashMap<String, String>,
}

/// Check a parsed PHP program for function calls to rename
pub fn check_rename_function<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_rename_function_with_config(program, source, &RenameFunctionConfig::default())
}

/// Check a parsed PHP program for function calls to rename with configuration
pub fn check_rename_function_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &RenameFunctionConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = RenameFunctionVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RenameFunctionVisitor<'s, 'c> {
    source: &'s str,
    config: &'c RenameFunctionConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> RenameFunctionVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's, 'c> Visitor<'a> for RenameFunctionVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            self.check_function_call(call);
        }
        true // Continue traversal
    }
}

impl<'s, 'c> RenameFunctionVisitor<'s, 'c> {
    fn check_function_call(&mut self, call: &FunctionCall<'_>) {
        // Get the function name
        let func_name = match &call.function {
            Expression::Identifier(ident) => self.get_text(ident.span()),
            _ => return, // Skip dynamic/variable function calls
        };

        // Look up in mappings (case-insensitive)
        let func_name_lower = func_name.to_lowercase();

        // Find matching mapping
        let new_name = self.config.mappings.iter().find_map(|(old, new)| {
            if old.to_lowercase() == func_name_lower {
                Some(new.clone())
            } else {
                None
            }
        });

        if let Some(new_name) = new_name {
            // Get the span of just the function name (not the whole call)
            let name_span = call.function.span();

            self.edits.push(Edit::new(
                name_span,
                new_name.clone(),
                format!("Rename function {} to {}", func_name, new_name),
            ));
        }
    }
}

/// Rule to rename function calls based on configuration
pub struct RenameFunctionRule {
    config: RenameFunctionConfig,
}

impl RenameFunctionRule {
    /// Create a new rule with default (empty) configuration
    pub fn new() -> Self {
        Self {
            config: RenameFunctionConfig::default(),
        }
    }

    /// Create a new rule with the given mappings
    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        Self {
            config: RenameFunctionConfig { mappings },
        }
    }
}

impl Default for RenameFunctionRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RenameFunctionRule {
    fn name(&self) -> &'static str {
        "rename_function"
    }

    fn description(&self) -> &'static str {
        "Rename function calls based on configurable mapping"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_rename_function_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None // No minimum version - depends on what functions are being renamed
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[ConfigOption {
            name: "mappings",
            description: "Map of old function names to new function names. Example: { \"old_func\" = \"new_func\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for RenameFunctionRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let mappings = config
            .get("mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        Self {
            config: RenameFunctionConfig { mappings },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &RenameFunctionConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_rename_function_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &RenameFunctionConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str)]) -> RenameFunctionConfig {
        RenameFunctionConfig {
            mappings: mappings
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_simple_rename() {
        let source = r#"<?php
$result = old_func($arg);
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("new_func($arg)"));
        assert!(!result.contains("old_func"));
    }

    #[test]
    fn test_multiple_mappings() {
        let source = r#"<?php
$a = legacy_one();
$b = legacy_two($x);
$c = keep_this();
"#;
        let config = make_config(&[
            ("legacy_one", "modern_one"),
            ("legacy_two", "modern_two"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);

        let result = transform_with_config(source, &config);
        assert!(result.contains("modern_one()"));
        assert!(result.contains("modern_two($x)"));
        assert!(result.contains("keep_this()")); // unchanged
    }

    #[test]
    fn test_case_insensitive_matching() {
        let source = r#"<?php
$a = OLD_FUNC();
$b = Old_Func();
$c = old_func();
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 3);

        let result = transform_with_config(source, &config);
        // All should be renamed to new_func
        assert_eq!(result.matches("new_func").count(), 3);
    }

    #[test]
    fn test_with_complex_arguments() {
        let source = r#"<?php
$result = old_func($a, $b, ['key' => $value], function($x) { return $x * 2; });
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("new_func($a, $b, ['key' => $value], function($x) { return $x * 2; })"));
    }

    #[test]
    fn test_nested_function_calls() {
        let source = r#"<?php
$result = outer_func(inner_func($arg));
"#;
        let config = make_config(&[
            ("outer_func", "new_outer"),
            ("inner_func", "new_inner"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);

        let result = transform_with_config(source, &config);
        assert!(result.contains("new_outer(new_inner($arg))"));
    }

    #[test]
    fn test_in_class_method() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return old_func($this->value);
    }
}
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("new_func($this->value)"));
    }

    #[test]
    fn test_in_closure() {
        let source = r#"<?php
$fn = function($x) {
    return old_func($x);
};
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_arrow_function() {
        let source = r#"<?php
$fn = fn($x) => old_func($x);
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_method_calls() {
        let source = r#"<?php
$obj->old_func();
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty(), "Should skip method calls");
    }

    #[test]
    fn test_skip_static_method_calls() {
        let source = r#"<?php
Foo::old_func();
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty(), "Should skip static method calls");
    }

    #[test]
    fn test_skip_dynamic_function_calls() {
        let source = r#"<?php
$func_name = 'old_func';
$func_name();
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty(), "Should skip variable function calls");
    }

    #[test]
    fn test_skip_unmatched_functions() {
        let source = r#"<?php
$a = some_func();
$b = other_func();
"#;
        let config = make_config(&[("old_func", "new_func")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty(), "Should skip unmatched functions");
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$a = old_func();
"#;
        let config = RenameFunctionConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty(), "Empty config should produce no edits");
    }

    // ==================== Configuration Tests ====================

    #[test]
    fn test_configurable_rule_with_config() {
        let mut config = HashMap::new();
        let mut mappings = HashMap::new();
        mappings.insert("old_func".to_string(), "new_func".to_string());
        config.insert("mappings".to_string(), ConfigValue::StringMap(mappings));

        let rule = RenameFunctionRule::with_config(&config);
        assert_eq!(rule.config.mappings.get("old_func"), Some(&"new_func".to_string()));
    }

    #[test]
    fn test_config_options_metadata() {
        let rule = RenameFunctionRule::new();
        let options = rule.config_options();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].name, "mappings");
        assert_eq!(options[0].option_type, ConfigOptionType::StringMap);
    }

    #[test]
    fn test_with_mappings_constructor() {
        let mut mappings = HashMap::new();
        mappings.insert("foo".to_string(), "bar".to_string());

        let rule = RenameFunctionRule::with_mappings(mappings);
        assert_eq!(rule.config.mappings.get("foo"), Some(&"bar".to_string()));
    }

    // ==================== Real-World Examples ====================

    #[test]
    fn test_php_deprecation_rename() {
        // Example: PHP deprecated function renames
        let source = r#"<?php
$encoded = utf8_encode($data);
$decoded = utf8_decode($encoded);
"#;
        let config = make_config(&[
            ("utf8_encode", "mb_convert_encoding"),
            ("utf8_decode", "mb_convert_encoding"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_framework_migration() {
        // Example: Framework function renames during migration
        let source = r#"<?php
$value = env('APP_DEBUG');
$config = config('app.name');
"#;
        let config = make_config(&[
            ("env", "getenv"),
            ("config", "app_config"),
        ]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("getenv('APP_DEBUG')"));
        assert!(result.contains("app_config('app.name')"));
    }
}
