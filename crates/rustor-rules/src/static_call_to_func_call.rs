//! Rule: static_call_to_func_call (Configurable)
//!
//! Transforms static method calls to function calls.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.static_call_to_func_call]
//! mappings = { "OldClass::method" = "new_function", "Str::slug" = "str_slug" }
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! $result = OldClass::method($arg);
//! $slug = Str::slug($text);
//!
//! // After
//! $result = new_function($arg);
//! $slug = str_slug($text);
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// A single static call to function mapping
#[derive(Debug, Clone)]
pub struct StaticToFuncMapping {
    pub class_name: String,
    pub method_name: String,
    pub func_name: String,
}

/// Configuration for the static_call_to_func_call rule
#[derive(Debug, Clone, Default)]
pub struct StaticCallToFuncCallConfig {
    pub mappings: Vec<StaticToFuncMapping>,
}

pub fn check_static_call_to_func_call<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_static_call_to_func_call_with_config(program, source, &StaticCallToFuncCallConfig::default())
}

pub fn check_static_call_to_func_call_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &StaticCallToFuncCallConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = StaticCallToFuncCallVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StaticCallToFuncCallVisitor<'s, 'c> {
    source: &'s str,
    config: &'c StaticCallToFuncCallConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> StaticCallToFuncCallVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's, 'c> Visitor<'a> for StaticCallToFuncCallVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::StaticMethod(static_call)) = expr {
            self.check_static_call(static_call, expr.span());
        }
        true
    }
}

impl<'s, 'c> StaticCallToFuncCallVisitor<'s, 'c> {
    fn check_static_call(&mut self, call: &StaticMethodCall<'_>, full_span: mago_span::Span) {
        // Get the class name
        let class_name = match &call.class {
            Expression::Identifier(ident) => self.get_text(ident.span()),
            _ => return,
        };

        // Get the method name
        let method_name = match &call.method {
            ClassLikeMemberSelector::Identifier(ident) => self.get_text(ident.span()),
            _ => return,
        };

        // Look up in mappings (case-insensitive for class, case-sensitive for method)
        let class_lower = class_name.to_lowercase();

        let mapping = self.config.mappings.iter().find(|m| {
            m.class_name.to_lowercase() == class_lower && m.method_name == method_name
        });

        if let Some(mapping) = mapping {
            let args_text = self.get_text(call.argument_list.span());
            let replacement = format!("{}{}", mapping.func_name, args_text);

            self.edits.push(Edit::new(
                full_span,
                replacement.clone(),
                format!("Replace {}::{}() with {}()", class_name, method_name, mapping.func_name),
            ));
        }
    }
}

pub struct StaticCallToFuncCallRule {
    config: StaticCallToFuncCallConfig,
}

impl StaticCallToFuncCallRule {
    pub fn new() -> Self {
        Self { config: StaticCallToFuncCallConfig::default() }
    }

    pub fn with_mappings(mappings: Vec<StaticToFuncMapping>) -> Self {
        Self { config: StaticCallToFuncCallConfig { mappings } }
    }

    /// Parse "Class::method" => "func_name" format
    pub fn from_string_map(map: HashMap<String, String>) -> Self {
        let mappings = map
            .into_iter()
            .filter_map(|(source, func)| {
                let parts: Vec<&str> = source.split("::").collect();
                if parts.len() == 2 {
                    Some(StaticToFuncMapping {
                        class_name: parts[0].to_string(),
                        method_name: parts[1].to_string(),
                        func_name: func,
                    })
                } else {
                    None
                }
            })
            .collect();

        Self { config: StaticCallToFuncCallConfig { mappings } }
    }
}

impl Default for StaticCallToFuncCallRule {
    fn default() -> Self { Self::new() }
}

impl Rule for StaticCallToFuncCallRule {
    fn name(&self) -> &'static str { "static_call_to_func_call" }
    fn description(&self) -> &'static str { "Transform static method calls to function calls" }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_static_call_to_func_call_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category { Category::Modernization }
    fn min_php_version(&self) -> Option<PhpVersion> { None }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[ConfigOption {
            name: "mappings",
            description: "Map of Class::method to function name. Example: { \"Str::slug\" = \"str_slug\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for StaticCallToFuncCallRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let mappings = config
            .get("mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();
        Self::from_string_map(mappings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &StaticCallToFuncCallConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_static_call_to_func_call_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &StaticCallToFuncCallConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str, &str)]) -> StaticCallToFuncCallConfig {
        StaticCallToFuncCallConfig {
            mappings: mappings.iter().map(|(class, method, func)| StaticToFuncMapping {
                class_name: class.to_string(),
                method_name: method.to_string(),
                func_name: func.to_string(),
            }).collect(),
        }
    }

    #[test]
    fn test_str_slug() {
        let source = r#"<?php
$slug = Str::slug($text);
"#;
        let config = make_config(&[("Str", "slug", "str_slug")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("str_slug($text)"));
    }

    #[test]
    fn test_old_class_method() {
        let source = r#"<?php
$result = OldClass::method($arg);
"#;
        let config = make_config(&[("OldClass", "method", "new_function")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("new_function($arg)"));
    }

    #[test]
    fn test_no_args() {
        let source = r#"<?php
$result = Util::now();
"#;
        let config = make_config(&[("Util", "now", "time")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("time()"));
    }

    #[test]
    fn test_multiple_args() {
        let source = r#"<?php
$result = Math::add($a, $b, $c);
"#;
        let config = make_config(&[("Math", "add", "array_sum")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("array_sum($a, $b, $c)"));
    }

    #[test]
    fn test_multiple_transformations() {
        let source = r#"<?php
$a = Str::slug($text);
$b = Str::upper($text);
"#;
        let config = make_config(&[
            ("Str", "slug", "str_slug"),
            ("Str", "upper", "strtoupper"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_case_insensitive_class() {
        let source = r#"<?php
$a = STR::slug($text);
$b = str::slug($text);
"#;
        let config = make_config(&[("Str", "slug", "str_slug")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_wrong_method() {
        let source = r#"<?php
$result = Str::other($text);
"#;
        let config = make_config(&[("Str", "slug", "str_slug")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$result = Other::slug($text);
"#;
        let config = make_config(&[("Str", "slug", "str_slug")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$slug = Str::slug($text);
"#;
        let config = StaticCallToFuncCallConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_from_string_map() {
        let mut map = HashMap::new();
        map.insert("Str::slug".to_string(), "str_slug".to_string());
        let rule = StaticCallToFuncCallRule::from_string_map(map);
        assert_eq!(rule.config.mappings.len(), 1);
        assert_eq!(rule.config.mappings[0].class_name, "Str");
        assert_eq!(rule.config.mappings[0].method_name, "slug");
        assert_eq!(rule.config.mappings[0].func_name, "str_slug");
    }
}
