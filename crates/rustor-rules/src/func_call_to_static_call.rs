//! Rule: func_call_to_static_call (Configurable)
//!
//! Transforms function calls to static method calls.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.func_call_to_static_call]
//! mappings = [
//!     { func = "view", class = "View", method = "render" },
//!     { func = "cache", class = "Cache", method = "get" }
//! ]
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! $html = view('home', $data);
//! $value = cache('key');
//!
//! // After
//! $html = View::render('home', $data);
//! $value = Cache::get('key');
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// A single function to static call mapping
#[derive(Debug, Clone)]
pub struct FuncToStaticMapping {
    pub func_name: String,
    pub class_name: String,
    pub method_name: String,
}

/// Configuration for the func_call_to_static_call rule
#[derive(Debug, Clone, Default)]
pub struct FuncCallToStaticCallConfig {
    /// List of mappings from function names to static calls
    pub mappings: Vec<FuncToStaticMapping>,
}

/// Check a parsed PHP program for function calls to replace with static calls
pub fn check_func_call_to_static_call<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_func_call_to_static_call_with_config(program, source, &FuncCallToStaticCallConfig::default())
}

/// Check with configuration
pub fn check_func_call_to_static_call_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &FuncCallToStaticCallConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = FuncCallToStaticCallVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct FuncCallToStaticCallVisitor<'s, 'c> {
    source: &'s str,
    config: &'c FuncCallToStaticCallConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> FuncCallToStaticCallVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's, 'c> Visitor<'a> for FuncCallToStaticCallVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            self.check_function_call(call, expr.span());
        }
        true
    }
}

impl<'s, 'c> FuncCallToStaticCallVisitor<'s, 'c> {
    fn check_function_call(&mut self, call: &FunctionCall<'_>, full_span: mago_span::Span) {
        // Get the function name
        let func_name = match &call.function {
            Expression::Identifier(ident) => self.get_text(ident.span()),
            _ => return, // Skip dynamic/variable function calls
        };

        // Look up in mappings (case-insensitive)
        let func_name_lower = func_name.to_lowercase();

        let mapping = self.config.mappings.iter().find(|m| {
            m.func_name.to_lowercase() == func_name_lower
        });

        if let Some(mapping) = mapping {
            // Get arguments text
            let args_text = self.get_text(call.argument_list.span());

            // Build static call: ClassName::methodName(args)
            let replacement = format!("{}::{}{}", mapping.class_name, mapping.method_name, args_text);

            self.edits.push(Edit::new(
                full_span,
                replacement.clone(),
                format!("Replace {}() with {}::{}()", func_name, mapping.class_name, mapping.method_name),
            ));
        }
    }
}

/// Rule to transform function calls to static method calls
pub struct FuncCallToStaticCallRule {
    config: FuncCallToStaticCallConfig,
}

impl FuncCallToStaticCallRule {
    pub fn new() -> Self {
        Self {
            config: FuncCallToStaticCallConfig::default(),
        }
    }

    pub fn with_mappings(mappings: Vec<FuncToStaticMapping>) -> Self {
        Self {
            config: FuncCallToStaticCallConfig { mappings },
        }
    }

    /// Convenience constructor using a simple string map format: "func" => "Class::method"
    pub fn from_string_map(map: HashMap<String, String>) -> Self {
        let mappings = map
            .into_iter()
            .filter_map(|(func, target)| {
                // Parse "Class::method" format
                let parts: Vec<&str> = target.split("::").collect();
                if parts.len() == 2 {
                    Some(FuncToStaticMapping {
                        func_name: func,
                        class_name: parts[0].to_string(),
                        method_name: parts[1].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Self {
            config: FuncCallToStaticCallConfig { mappings },
        }
    }
}

impl Default for FuncCallToStaticCallRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for FuncCallToStaticCallRule {
    fn name(&self) -> &'static str {
        "func_call_to_static_call"
    }

    fn description(&self) -> &'static str {
        "Transform function calls to static method calls"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_func_call_to_static_call_with_config(program, source, &self.config)
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
            description: "Map of function names to static calls. Example: { \"view\" = \"View::render\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for FuncCallToStaticCallRule {
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

    fn check_php_with_config(source: &str, config: &FuncCallToStaticCallConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_func_call_to_static_call_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &FuncCallToStaticCallConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str, &str)]) -> FuncCallToStaticCallConfig {
        FuncCallToStaticCallConfig {
            mappings: mappings
                .iter()
                .map(|(func, class, method)| FuncToStaticMapping {
                    func_name: func.to_string(),
                    class_name: class.to_string(),
                    method_name: method.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_view_to_static() {
        let source = r#"<?php
$html = view('home', $data);
"#;
        let config = make_config(&[("view", "View", "render")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("View::render('home', $data)"));
    }

    #[test]
    fn test_cache_to_static() {
        let source = r#"<?php
$value = cache('key');
"#;
        let config = make_config(&[("cache", "Cache", "get")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Cache::get('key')"));
    }

    #[test]
    fn test_no_args() {
        let source = r#"<?php
$result = helper();
"#;
        let config = make_config(&[("helper", "Helper", "call")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Helper::call()"));
    }

    #[test]
    fn test_multiple_transformations() {
        let source = r#"<?php
$a = view('home');
$b = cache('key');
"#;
        let config = make_config(&[
            ("view", "View", "render"),
            ("cache", "Cache", "get"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);

        let result = transform_with_config(source, &config);
        assert!(result.contains("View::render('home')"));
        assert!(result.contains("Cache::get('key')"));
    }

    #[test]
    fn test_case_insensitive() {
        let source = r#"<?php
$a = VIEW('home');
$b = View('home');
"#;
        let config = make_config(&[("view", "View", "render")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$result = other_func();
"#;
        let config = make_config(&[("view", "View", "render")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$html = view('home');
"#;
        let config = FuncCallToStaticCallConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_from_string_map() {
        let mut map = HashMap::new();
        map.insert("view".to_string(), "View::render".to_string());

        let rule = FuncCallToStaticCallRule::from_string_map(map);
        assert_eq!(rule.config.mappings.len(), 1);
        assert_eq!(rule.config.mappings[0].func_name, "view");
        assert_eq!(rule.config.mappings[0].class_name, "View");
        assert_eq!(rule.config.mappings[0].method_name, "render");
    }

    #[test]
    fn test_in_class() {
        let source = r#"<?php
class Controller {
    public function index() {
        return view('home');
    }
}
"#;
        let config = make_config(&[("view", "View", "render")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_namespaced_class() {
        let source = r#"<?php
$html = view('home');
"#;
        let config = make_config(&[("view", "App\\Views\\ViewFactory", "make")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("App\\Views\\ViewFactory::make('home')"));
    }
}
