//! Rule: func_call_to_const_fetch (Configurable)
//!
//! Transforms function calls to constant fetches.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.func_call_to_const_fetch]
//! mappings = { "php_sapi_name" = "PHP_SAPI", "phpversion" = "PHP_VERSION" }
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! $sapi = php_sapi_name();
//! $version = phpversion();
//!
//! // After
//! $sapi = PHP_SAPI;
//! $version = PHP_VERSION;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// Configuration for the func_call_to_const_fetch rule
#[derive(Debug, Clone, Default)]
pub struct FuncCallToConstFetchConfig {
    /// Map of function names to constant names
    pub mappings: HashMap<String, String>,
}

/// Check a parsed PHP program for function calls to replace with constants
pub fn check_func_call_to_const_fetch<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_func_call_to_const_fetch_with_config(program, source, &FuncCallToConstFetchConfig::default())
}

/// Check a parsed PHP program for function calls to replace with constants
pub fn check_func_call_to_const_fetch_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &FuncCallToConstFetchConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = FuncCallToConstFetchVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct FuncCallToConstFetchVisitor<'s, 'c> {
    source: &'s str,
    config: &'c FuncCallToConstFetchConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> FuncCallToConstFetchVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's, 'c> Visitor<'a> for FuncCallToConstFetchVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            self.check_function_call(call, expr.span());
        }
        true
    }
}

impl<'s, 'c> FuncCallToConstFetchVisitor<'s, 'c> {
    fn check_function_call(&mut self, call: &FunctionCall<'_>, full_span: mago_span::Span) {
        // Only transform function calls with no arguments
        if !call.argument_list.arguments.is_empty() {
            return;
        }

        // Get the function name
        let func_name = match &call.function {
            Expression::Identifier(ident) => self.get_text(ident.span()),
            _ => return, // Skip dynamic/variable function calls
        };

        // Look up in mappings (case-insensitive)
        let func_name_lower = func_name.to_lowercase();

        let const_name = self.config.mappings.iter().find_map(|(old, new)| {
            if old.to_lowercase() == func_name_lower {
                Some(new.clone())
            } else {
                None
            }
        });

        if let Some(const_name) = const_name {
            self.edits.push(Edit::new(
                full_span,
                const_name.clone(),
                format!("Replace {}() with {}", func_name, const_name),
            ));
        }
    }
}

/// Rule to transform function calls to constants
pub struct FuncCallToConstFetchRule {
    config: FuncCallToConstFetchConfig,
}

impl FuncCallToConstFetchRule {
    pub fn new() -> Self {
        Self {
            config: FuncCallToConstFetchConfig::default(),
        }
    }

    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        Self {
            config: FuncCallToConstFetchConfig { mappings },
        }
    }
}

impl Default for FuncCallToConstFetchRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for FuncCallToConstFetchRule {
    fn name(&self) -> &'static str {
        "func_call_to_const_fetch"
    }

    fn description(&self) -> &'static str {
        "Transform function calls to constant fetches"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_func_call_to_const_fetch_with_config(program, source, &self.config)
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
            description: "Map of function names to constant names. Example: { \"php_sapi_name\" = \"PHP_SAPI\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for FuncCallToConstFetchRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let mappings = config
            .get("mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        Self {
            config: FuncCallToConstFetchConfig { mappings },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &FuncCallToConstFetchConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_func_call_to_const_fetch_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &FuncCallToConstFetchConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str)]) -> FuncCallToConstFetchConfig {
        FuncCallToConstFetchConfig {
            mappings: mappings
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn test_php_sapi_name() {
        let source = r#"<?php
$sapi = php_sapi_name();
"#;
        let config = make_config(&[("php_sapi_name", "PHP_SAPI")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("$sapi = PHP_SAPI;"));
    }

    #[test]
    fn test_phpversion() {
        let source = r#"<?php
$version = phpversion();
"#;
        let config = make_config(&[("phpversion", "PHP_VERSION")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("$version = PHP_VERSION;"));
    }

    #[test]
    fn test_multiple_transformations() {
        let source = r#"<?php
$sapi = php_sapi_name();
$version = phpversion();
"#;
        let config = make_config(&[
            ("php_sapi_name", "PHP_SAPI"),
            ("phpversion", "PHP_VERSION"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);

        let result = transform_with_config(source, &config);
        assert!(result.contains("PHP_SAPI"));
        assert!(result.contains("PHP_VERSION"));
    }

    #[test]
    fn test_case_insensitive() {
        let source = r#"<?php
$a = PHP_SAPI_NAME();
$b = Php_Sapi_Name();
"#;
        let config = make_config(&[("php_sapi_name", "PHP_SAPI")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_function_with_args() {
        let source = r#"<?php
// php_sapi_name doesn't take args, but if it did, skip it
$result = php_sapi_name("arg");
"#;
        let config = make_config(&[("php_sapi_name", "PHP_SAPI")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$result = some_other_func();
"#;
        let config = make_config(&[("php_sapi_name", "PHP_SAPI")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$sapi = php_sapi_name();
"#;
        let config = FuncCallToConstFetchConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_in_class() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return php_sapi_name();
    }
}
"#;
        let config = make_config(&[("php_sapi_name", "PHP_SAPI")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }
}
