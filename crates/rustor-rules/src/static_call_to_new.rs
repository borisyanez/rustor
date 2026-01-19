//! Rule: static_call_to_new (Configurable)
//!
//! Transforms static method calls to new instance creations.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.static_call_to_new]
//! mappings = { "JsonResponse::create" = "JsonResponse", "Cookie::make" = "Cookie" }
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! $response = JsonResponse::create(['foo' => 'bar']);
//! $cookie = Cookie::make($name);
//!
//! // After
//! $response = new JsonResponse(['foo' => 'bar']);
//! $cookie = new Cookie($name);
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// A single static call to new mapping
#[derive(Debug, Clone)]
pub struct StaticToNewMapping {
    pub class_name: String,
    pub method_name: String,
    pub target_class: String,
}

/// Configuration for the static_call_to_new rule
#[derive(Debug, Clone, Default)]
pub struct StaticCallToNewConfig {
    pub mappings: Vec<StaticToNewMapping>,
}

pub fn check_static_call_to_new<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_static_call_to_new_with_config(program, source, &StaticCallToNewConfig::default())
}

pub fn check_static_call_to_new_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &StaticCallToNewConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = StaticCallToNewVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StaticCallToNewVisitor<'s, 'c> {
    source: &'s str,
    config: &'c StaticCallToNewConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> StaticCallToNewVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's, 'c> Visitor<'a> for StaticCallToNewVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::StaticMethod(static_call)) = expr {
            self.check_static_call(static_call, expr.span());
        }
        true
    }
}

impl<'s, 'c> StaticCallToNewVisitor<'s, 'c> {
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
            let replacement = format!("new {}{}", mapping.target_class, args_text);

            self.edits.push(Edit::new(
                full_span,
                replacement.clone(),
                format!("Replace {}::{}() with new {}()", class_name, method_name, mapping.target_class),
            ));
        }
    }
}

pub struct StaticCallToNewRule {
    config: StaticCallToNewConfig,
}

impl StaticCallToNewRule {
    pub fn new() -> Self {
        Self { config: StaticCallToNewConfig::default() }
    }

    pub fn with_mappings(mappings: Vec<StaticToNewMapping>) -> Self {
        Self { config: StaticCallToNewConfig { mappings } }
    }

    /// Parse "Class::method" => "TargetClass" format
    pub fn from_string_map(map: HashMap<String, String>) -> Self {
        let mappings = map
            .into_iter()
            .filter_map(|(source, target)| {
                let parts: Vec<&str> = source.split("::").collect();
                if parts.len() == 2 {
                    Some(StaticToNewMapping {
                        class_name: parts[0].to_string(),
                        method_name: parts[1].to_string(),
                        target_class: target,
                    })
                } else {
                    None
                }
            })
            .collect();

        Self { config: StaticCallToNewConfig { mappings } }
    }
}

impl Default for StaticCallToNewRule {
    fn default() -> Self { Self::new() }
}

impl Rule for StaticCallToNewRule {
    fn name(&self) -> &'static str { "static_call_to_new" }
    fn description(&self) -> &'static str { "Transform static method calls to new instance creations" }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_static_call_to_new_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category { Category::Modernization }
    fn min_php_version(&self) -> Option<PhpVersion> { None }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[ConfigOption {
            name: "mappings",
            description: "Map of Class::method to target class. Example: { \"JsonResponse::create\" = \"JsonResponse\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for StaticCallToNewRule {
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

    fn check_php_with_config(source: &str, config: &StaticCallToNewConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_static_call_to_new_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &StaticCallToNewConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str, &str)]) -> StaticCallToNewConfig {
        StaticCallToNewConfig {
            mappings: mappings.iter().map(|(class, method, target)| StaticToNewMapping {
                class_name: class.to_string(),
                method_name: method.to_string(),
                target_class: target.to_string(),
            }).collect(),
        }
    }

    #[test]
    fn test_json_response_create() {
        let source = r#"<?php
$resp = JsonResponse::create(['foo' => 'bar']);
"#;
        let config = make_config(&[("JsonResponse", "create", "JsonResponse")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("new JsonResponse(['foo' => 'bar'])"));
    }

    #[test]
    fn test_different_target() {
        let source = r#"<?php
$obj = OldClass::make($arg);
"#;
        let config = make_config(&[("OldClass", "make", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("new NewClass($arg)"));
    }

    #[test]
    fn test_no_args() {
        let source = r#"<?php
$obj = Factory::instance();
"#;
        let config = make_config(&[("Factory", "instance", "Factory")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("new Factory()"));
    }

    #[test]
    fn test_multiple() {
        let source = r#"<?php
$a = JsonResponse::create([]);
$b = Cookie::make($name);
"#;
        let config = make_config(&[
            ("JsonResponse", "create", "JsonResponse"),
            ("Cookie", "make", "Cookie"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_case_insensitive_class() {
        let source = r#"<?php
$a = JSONRESPONSE::create([]);
$b = jsonresponse::create([]);
"#;
        let config = make_config(&[("JsonResponse", "create", "JsonResponse")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_wrong_method() {
        let source = r#"<?php
$obj = JsonResponse::other([]);
"#;
        let config = make_config(&[("JsonResponse", "create", "JsonResponse")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$obj = OtherClass::create([]);
"#;
        let config = make_config(&[("JsonResponse", "create", "JsonResponse")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$obj = JsonResponse::create([]);
"#;
        let config = StaticCallToNewConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_from_string_map() {
        let mut map = HashMap::new();
        map.insert("JsonResponse::create".to_string(), "JsonResponse".to_string());
        let rule = StaticCallToNewRule::from_string_map(map);
        assert_eq!(rule.config.mappings.len(), 1);
        assert_eq!(rule.config.mappings[0].class_name, "JsonResponse");
        assert_eq!(rule.config.mappings[0].method_name, "create");
    }
}
