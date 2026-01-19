//! Rule: scalar_value_to_const_fetch (Configurable)
//!
//! Transforms scalar values (int, float, string) to class constants or global constants.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.scalar_value_to_const_fetch]
//! int_mappings = { "10" = "SomeClass::FOOBAR_INT", "200" = "Response::HTTP_OK" }
//! string_mappings = { "utf-8" = "Encoding::UTF8" }
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! $var = 10;
//! $status = 200;
//!
//! // After
//! $var = SomeClass::FOOBAR_INT;
//! $status = Response::HTTP_OK;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// Configuration for the scalar_value_to_const_fetch rule
#[derive(Debug, Clone, Default)]
pub struct ScalarValueToConstFetchConfig {
    /// Map of integer values to class constants
    pub int_mappings: HashMap<i64, String>,
    /// Map of float values to class constants (stored as string for exact matching)
    pub float_mappings: HashMap<String, String>,
    /// Map of string values to class constants
    pub string_mappings: HashMap<String, String>,
}

pub fn check_scalar_value_to_const_fetch<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_scalar_value_to_const_fetch_with_config(program, source, &ScalarValueToConstFetchConfig::default())
}

pub fn check_scalar_value_to_const_fetch_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &ScalarValueToConstFetchConfig,
) -> Vec<Edit> {
    if config.int_mappings.is_empty() && config.float_mappings.is_empty() && config.string_mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = ScalarValueToConstFetchVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ScalarValueToConstFetchVisitor<'s, 'c> {
    source: &'s str,
    config: &'c ScalarValueToConstFetchConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> ScalarValueToConstFetchVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's, 'c> Visitor<'a> for ScalarValueToConstFetchVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        match expr {
            Expression::Literal(Literal::Integer(int_lit)) => {
                self.check_integer(int_lit);
            }
            Expression::Literal(Literal::Float(float_lit)) => {
                self.check_float(float_lit);
            }
            Expression::Literal(Literal::String(string_lit)) => {
                self.check_string(string_lit);
            }
            _ => {}
        }
        true
    }
}

impl<'s, 'c> ScalarValueToConstFetchVisitor<'s, 'c> {
    fn check_integer(&mut self, int_lit: &LiteralInteger<'_>) {
        let text = self.get_text(int_lit.span());

        // Parse the integer value
        let value = if let Some(stripped) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            i64::from_str_radix(stripped, 16).ok()
        } else if let Some(stripped) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
            i64::from_str_radix(stripped, 2).ok()
        } else if let Some(stripped) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
            i64::from_str_radix(stripped, 8).ok()
        } else if text.starts_with('0') && text.len() > 1 && !text.contains('.') {
            // Legacy octal (0777)
            i64::from_str_radix(&text[1..], 8).ok()
        } else {
            text.replace('_', "").parse::<i64>().ok()
        };

        if let Some(value) = value {
            if let Some(replacement) = self.config.int_mappings.get(&value) {
                self.edits.push(Edit::new(
                    int_lit.span(),
                    replacement.clone(),
                    format!("Replace {} with {}", text, replacement),
                ));
            }
        }
    }

    fn check_float(&mut self, float_lit: &LiteralFloat<'_>) {
        let text = self.get_text(float_lit.span());
        let normalized = text.replace('_', "");

        if let Some(replacement) = self.config.float_mappings.get(&normalized) {
            self.edits.push(Edit::new(
                float_lit.span(),
                replacement.clone(),
                format!("Replace {} with {}", text, replacement),
            ));
        }
    }

    fn check_string(&mut self, string_lit: &LiteralString<'_>) {
        let full_text = self.get_text(string_lit.span());

        // Get quote character
        let quote_char = match full_text.chars().next() {
            Some(c) if c == '\'' || c == '"' => c,
            _ => return, // Heredoc/nowdoc - skip
        };

        if full_text.len() < 2 {
            return;
        }

        // Extract string content without quotes
        let content = &full_text[1..full_text.len() - 1];

        if let Some(replacement) = self.config.string_mappings.get(content) {
            self.edits.push(Edit::new(
                string_lit.span(),
                replacement.clone(),
                format!("Replace {}{}{} with {}", quote_char, content, quote_char, replacement),
            ));
        }
    }
}

pub struct ScalarValueToConstFetchRule {
    config: ScalarValueToConstFetchConfig,
}

impl ScalarValueToConstFetchRule {
    pub fn new() -> Self {
        Self { config: ScalarValueToConstFetchConfig::default() }
    }

    pub fn with_config_struct(config: ScalarValueToConstFetchConfig) -> Self {
        Self { config }
    }

    /// Parse from string maps: "10" => "SomeClass::CONST"
    pub fn from_string_maps(
        int_map: HashMap<String, String>,
        float_map: HashMap<String, String>,
        string_map: HashMap<String, String>,
    ) -> Self {
        let int_mappings: HashMap<i64, String> = int_map
            .into_iter()
            .filter_map(|(k, v)| k.parse::<i64>().ok().map(|n| (n, v)))
            .collect();

        Self {
            config: ScalarValueToConstFetchConfig {
                int_mappings,
                float_mappings: float_map,
                string_mappings: string_map,
            },
        }
    }
}

impl Default for ScalarValueToConstFetchRule {
    fn default() -> Self { Self::new() }
}

impl Rule for ScalarValueToConstFetchRule {
    fn name(&self) -> &'static str { "scalar_value_to_const_fetch" }
    fn description(&self) -> &'static str { "Transform scalar values to class constants" }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_scalar_value_to_const_fetch_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category { Category::Modernization }
    fn min_php_version(&self) -> Option<PhpVersion> { None }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[
            ConfigOption {
                name: "int_mappings",
                description: "Map of integer values to constants. Example: { \"200\" = \"Response::HTTP_OK\" }",
                default: "{}",
                option_type: ConfigOptionType::StringMap,
            },
            ConfigOption {
                name: "float_mappings",
                description: "Map of float values to constants. Example: { \"3.14\" = \"Math::PI\" }",
                default: "{}",
                option_type: ConfigOptionType::StringMap,
            },
            ConfigOption {
                name: "string_mappings",
                description: "Map of string values to constants. Example: { \"utf-8\" = \"Encoding::UTF8\" }",
                default: "{}",
                option_type: ConfigOptionType::StringMap,
            },
        ];
        OPTIONS
    }
}

impl ConfigurableRule for ScalarValueToConstFetchRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let int_mappings = config
            .get("int_mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        let float_mappings = config
            .get("float_mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        let string_mappings = config
            .get("string_mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        Self::from_string_maps(int_mappings, float_mappings, string_mappings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &ScalarValueToConstFetchConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_scalar_value_to_const_fetch_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &ScalarValueToConstFetchConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_int_config(mappings: &[(i64, &str)]) -> ScalarValueToConstFetchConfig {
        ScalarValueToConstFetchConfig {
            int_mappings: mappings.iter().map(|(k, v)| (*k, v.to_string())).collect(),
            float_mappings: HashMap::new(),
            string_mappings: HashMap::new(),
        }
    }

    fn make_string_config(mappings: &[(&str, &str)]) -> ScalarValueToConstFetchConfig {
        ScalarValueToConstFetchConfig {
            int_mappings: HashMap::new(),
            float_mappings: HashMap::new(),
            string_mappings: mappings.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }

    #[test]
    fn test_integer_to_const() {
        let source = r#"<?php
$var = 10;
"#;
        let config = make_int_config(&[(10, "SomeClass::FOOBAR_INT")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("SomeClass::FOOBAR_INT"));
    }

    #[test]
    fn test_http_status() {
        let source = r#"<?php
$status = 200;
$notFound = 404;
"#;
        let config = make_int_config(&[(200, "Response::HTTP_OK"), (404, "Response::HTTP_NOT_FOUND")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
        let result = transform_with_config(source, &config);
        assert!(result.contains("Response::HTTP_OK"));
        assert!(result.contains("Response::HTTP_NOT_FOUND"));
    }

    #[test]
    fn test_string_to_const() {
        let source = r#"<?php
$encoding = 'utf-8';
"#;
        let config = make_string_config(&[("utf-8", "Encoding::UTF8")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("Encoding::UTF8"));
    }

    #[test]
    fn test_double_quoted_string() {
        let source = r#"<?php
$type = "integer";
"#;
        let config = make_string_config(&[("integer", "Types::INTEGER")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("Types::INTEGER"));
    }

    #[test]
    fn test_skip_unmatched_integer() {
        let source = r#"<?php
$var = 42;
"#;
        let config = make_int_config(&[(10, "SomeClass::TEN")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_unmatched_string() {
        let source = r#"<?php
$str = 'other';
"#;
        let config = make_string_config(&[("utf-8", "Encoding::UTF8")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$var = 10;
"#;
        let config = ScalarValueToConstFetchConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_occurrences() {
        let source = r#"<?php
$a = 200;
$b = 200;
"#;
        let config = make_int_config(&[(200, "Response::HTTP_OK")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_negative_integer() {
        let source = r#"<?php
$val = -1;
"#;
        // Note: negative numbers are typically represented as unary minus + positive int in the AST
        // so this won't match directly. The rule matches the literal "1" not "-1"
        let config = make_int_config(&[(1, "Constants::ONE")]);
        let edits = check_php_with_config(source, &config);
        // Should match the "1" in "-1"
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_from_string_maps() {
        let mut int_map = HashMap::new();
        int_map.insert("200".to_string(), "Response::HTTP_OK".to_string());

        let rule = ScalarValueToConstFetchRule::from_string_maps(int_map, HashMap::new(), HashMap::new());
        assert_eq!(rule.config.int_mappings.len(), 1);
        assert_eq!(rule.config.int_mappings.get(&200), Some(&"Response::HTTP_OK".to_string()));
    }
}
