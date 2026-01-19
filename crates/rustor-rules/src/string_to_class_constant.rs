//! Rule: string_to_class_constant (Configurable)
//!
//! Transforms string literals to class constants.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.string_to_class_constant]
//! mappings = { "compiler.post_dump" = "Compiler::POST_DUMP", "kernel.request" = "KernelEvents::REQUEST" }
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! return ['compiler.post_dump' => 'compile'];
//!
//! // After
//! return [Compiler::POST_DUMP => 'compile'];
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// A single string to class constant mapping
#[derive(Debug, Clone)]
pub struct StringToConstMapping {
    pub string_value: String,
    pub class_name: String,
    pub const_name: String,
}

/// Configuration for the string_to_class_constant rule
#[derive(Debug, Clone, Default)]
pub struct StringToClassConstantConfig {
    /// List of mappings from strings to class constants
    pub mappings: Vec<StringToConstMapping>,
}

/// Check a parsed PHP program for strings to replace with class constants
pub fn check_string_to_class_constant<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_string_to_class_constant_with_config(program, source, &StringToClassConstantConfig::default())
}

/// Check with configuration
pub fn check_string_to_class_constant_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &StringToClassConstantConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = StringToClassConstantVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StringToClassConstantVisitor<'s, 'c> {
    source: &'s str,
    config: &'c StringToClassConstantConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> StringToClassConstantVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Extract the string value from a string literal (removes quotes)
    fn extract_string_value(&self, literal: &LiteralString<'_>) -> Option<String> {
        let full_text = self.get_text(literal.span());

        // Get quote character
        let quote_char = full_text.chars().next()?;
        if quote_char != '\'' && quote_char != '"' {
            return None; // Heredoc/nowdoc - skip
        }

        // Get the string content (without quotes)
        if full_text.len() < 2 {
            return None;
        }

        Some(full_text[1..full_text.len() - 1].to_string())
    }
}

impl<'a, 's, 'c> Visitor<'a> for StringToClassConstantVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Literal(Literal::String(string_lit)) = expr {
            self.check_string_literal(string_lit, expr.span());
        }
        true
    }
}

impl<'s, 'c> StringToClassConstantVisitor<'s, 'c> {
    fn check_string_literal(&mut self, string_lit: &LiteralString<'_>, span: mago_span::Span) {
        let string_value = match self.extract_string_value(string_lit) {
            Some(v) => v,
            None => return,
        };

        // Look up in mappings
        let mapping = self.config.mappings.iter().find(|m| {
            m.string_value == string_value
        });

        if let Some(mapping) = mapping {
            let replacement = format!("{}::{}", mapping.class_name, mapping.const_name);

            self.edits.push(Edit::new(
                span,
                replacement.clone(),
                format!("Replace '{}' with {}", string_value, replacement),
            ));
        }
    }
}

/// Rule to transform strings to class constants
pub struct StringToClassConstantRule {
    config: StringToClassConstantConfig,
}

impl StringToClassConstantRule {
    pub fn new() -> Self {
        Self {
            config: StringToClassConstantConfig::default(),
        }
    }

    pub fn with_mappings(mappings: Vec<StringToConstMapping>) -> Self {
        Self {
            config: StringToClassConstantConfig { mappings },
        }
    }

    /// Convenience constructor using a simple string map format: "string" => "Class::CONST"
    pub fn from_string_map(map: HashMap<String, String>) -> Self {
        let mappings = map
            .into_iter()
            .filter_map(|(string, target)| {
                // Parse "Class::CONST" format
                let parts: Vec<&str> = target.split("::").collect();
                if parts.len() == 2 {
                    Some(StringToConstMapping {
                        string_value: string,
                        class_name: parts[0].to_string(),
                        const_name: parts[1].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Self {
            config: StringToClassConstantConfig { mappings },
        }
    }
}

impl Default for StringToClassConstantRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for StringToClassConstantRule {
    fn name(&self) -> &'static str {
        "string_to_class_constant"
    }

    fn description(&self) -> &'static str {
        "Transform string literals to class constants"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_string_to_class_constant_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[ConfigOption {
            name: "mappings",
            description: "Map of strings to class constants. Example: { \"event.name\" = \"Events::NAME\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for StringToClassConstantRule {
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

    fn check_php_with_config(source: &str, config: &StringToClassConstantConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_string_to_class_constant_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &StringToClassConstantConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str, &str)]) -> StringToClassConstantConfig {
        StringToClassConstantConfig {
            mappings: mappings
                .iter()
                .map(|(string, class, cnst)| StringToConstMapping {
                    string_value: string.to_string(),
                    class_name: class.to_string(),
                    const_name: cnst.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_event_name_to_constant() {
        let source = r#"<?php
return ['compiler.post_dump' => 'compile'];
"#;
        let config = make_config(&[("compiler.post_dump", "Compiler", "POST_DUMP")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Compiler::POST_DUMP"));
    }

    #[test]
    fn test_double_quoted_string() {
        let source = r#"<?php
$event = "kernel.request";
"#;
        let config = make_config(&[("kernel.request", "KernelEvents", "REQUEST")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("KernelEvents::REQUEST"));
    }

    #[test]
    fn test_in_array_key() {
        let source = r#"<?php
$subscribers = [
    'kernel.request' => 'onRequest',
    'kernel.response' => 'onResponse',
];
"#;
        let config = make_config(&[
            ("kernel.request", "KernelEvents", "REQUEST"),
            ("kernel.response", "KernelEvents", "RESPONSE"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);

        let result = transform_with_config(source, &config);
        assert!(result.contains("KernelEvents::REQUEST"));
        assert!(result.contains("KernelEvents::RESPONSE"));
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$str = 'some other string';
"#;
        let config = make_config(&[("kernel.request", "KernelEvents", "REQUEST")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$event = 'kernel.request';
"#;
        let config = StringToClassConstantConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_from_string_map() {
        let mut map = HashMap::new();
        map.insert("event.name".to_string(), "Events::NAME".to_string());

        let rule = StringToClassConstantRule::from_string_map(map);
        assert_eq!(rule.config.mappings.len(), 1);
        assert_eq!(rule.config.mappings[0].string_value, "event.name");
        assert_eq!(rule.config.mappings[0].class_name, "Events");
        assert_eq!(rule.config.mappings[0].const_name, "NAME");
    }

    #[test]
    fn test_namespaced_class() {
        let source = r#"<?php
$event = 'form.submit';
"#;
        let config = make_config(&[("form.submit", "Symfony\\Component\\Form\\FormEvents", "SUBMIT")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Symfony\\Component\\Form\\FormEvents::SUBMIT"));
    }

    #[test]
    fn test_case_sensitive() {
        // String matching should be case-sensitive
        let source = r#"<?php
$a = 'Kernel.Request';
$b = 'kernel.request';
"#;
        let config = make_config(&[("kernel.request", "KernelEvents", "REQUEST")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1); // Only $b matches
    }

    #[test]
    fn test_skip_string_values_not_keys() {
        // This tests that we replace string values that match, regardless of position
        let source = r#"<?php
return ['key' => 'kernel.request'];
"#;
        let config = make_config(&[("kernel.request", "KernelEvents", "REQUEST")]);
        let edits = check_php_with_config(source, &config);
        // The string value 'kernel.request' should be replaced
        assert_eq!(edits.len(), 1);
    }
}
