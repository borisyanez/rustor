//! Rule: new_to_static_call (Configurable)
//!
//! Transforms `new ClassName()` to static method calls.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.new_to_static_call]
//! mappings = { "Cookie" = "Cookie::create", "Response" = "Response::new" }
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! new Cookie($name);
//! new Response($data, 200);
//!
//! // After
//! Cookie::create($name);
//! Response::new($data, 200);
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// A single new to static call mapping
#[derive(Debug, Clone)]
pub struct NewToStaticMapping {
    pub class_name: String,
    pub target_class: String,
    pub target_method: String,
}

/// Configuration for the new_to_static_call rule
#[derive(Debug, Clone, Default)]
pub struct NewToStaticCallConfig {
    /// List of mappings from new ClassName() to static calls
    pub mappings: Vec<NewToStaticMapping>,
}

/// Check a parsed PHP program for new expressions to replace with static calls
pub fn check_new_to_static_call<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_new_to_static_call_with_config(program, source, &NewToStaticCallConfig::default())
}

/// Check with configuration
pub fn check_new_to_static_call_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &NewToStaticCallConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = NewToStaticCallVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct NewToStaticCallVisitor<'s, 'c> {
    source: &'s str,
    config: &'c NewToStaticCallConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> NewToStaticCallVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's, 'c> Visitor<'a> for NewToStaticCallVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Instantiation(new_expr) = expr {
            self.check_new_expression(new_expr, expr.span());
        }
        true
    }
}

impl<'s, 'c> NewToStaticCallVisitor<'s, 'c> {
    fn check_new_expression(&mut self, new_expr: &Instantiation<'_>, full_span: mago_span::Span) {
        // Get the class name from the new expression
        let class_name = match &new_expr.class {
            Expression::Identifier(ident) => self.get_text(ident.span()),
            _ => return, // Skip dynamic class instantiation
        };

        // Look up in mappings (case-insensitive for class names)
        let class_name_lower = class_name.to_lowercase();

        let mapping = self.config.mappings.iter().find(|m| {
            m.class_name.to_lowercase() == class_name_lower
        });

        if let Some(mapping) = mapping {
            // Get arguments text (if any)
            let args_text = new_expr.argument_list
                .as_ref()
                .map(|args| self.get_text(args.span()))
                .unwrap_or("()");

            // Build static call: TargetClass::method(args)
            let replacement = format!("{}::{}{}", mapping.target_class, mapping.target_method, args_text);

            self.edits.push(Edit::new(
                full_span,
                replacement.clone(),
                format!("Replace new {}() with {}::{}()", class_name, mapping.target_class, mapping.target_method),
            ));
        }
    }
}

/// Rule to transform new expressions to static method calls
pub struct NewToStaticCallRule {
    config: NewToStaticCallConfig,
}

impl NewToStaticCallRule {
    pub fn new() -> Self {
        Self {
            config: NewToStaticCallConfig::default(),
        }
    }

    pub fn with_mappings(mappings: Vec<NewToStaticMapping>) -> Self {
        Self {
            config: NewToStaticCallConfig { mappings },
        }
    }

    /// Convenience constructor using a simple string map format: "ClassName" => "TargetClass::method"
    pub fn from_string_map(map: HashMap<String, String>) -> Self {
        let mappings = map
            .into_iter()
            .filter_map(|(class, target)| {
                // Parse "TargetClass::method" format
                let parts: Vec<&str> = target.split("::").collect();
                if parts.len() == 2 {
                    Some(NewToStaticMapping {
                        class_name: class,
                        target_class: parts[0].to_string(),
                        target_method: parts[1].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Self {
            config: NewToStaticCallConfig { mappings },
        }
    }
}

impl Default for NewToStaticCallRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for NewToStaticCallRule {
    fn name(&self) -> &'static str {
        "new_to_static_call"
    }

    fn description(&self) -> &'static str {
        "Transform new ClassName() to static method calls"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_new_to_static_call_with_config(program, source, &self.config)
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
            description: "Map of class names to static calls. Example: { \"Cookie\" = \"Cookie::create\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for NewToStaticCallRule {
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

    fn check_php_with_config(source: &str, config: &NewToStaticCallConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_new_to_static_call_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &NewToStaticCallConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str, &str)]) -> NewToStaticCallConfig {
        NewToStaticCallConfig {
            mappings: mappings
                .iter()
                .map(|(class, target_class, method)| NewToStaticMapping {
                    class_name: class.to_string(),
                    target_class: target_class.to_string(),
                    target_method: method.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_cookie_to_create() {
        let source = r#"<?php
$cookie = new Cookie($name);
"#;
        let config = make_config(&[("Cookie", "Cookie", "create")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Cookie::create($name)"));
    }

    #[test]
    fn test_response_to_new() {
        let source = r#"<?php
$resp = new Response($data, 200);
"#;
        let config = make_config(&[("Response", "Response", "new")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Response::new($data, 200)"));
    }

    #[test]
    fn test_no_args() {
        let source = r#"<?php
$obj = new Factory();
"#;
        let config = make_config(&[("Factory", "Factory", "instance")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Factory::instance()"));
    }

    #[test]
    fn test_different_target_class() {
        let source = r#"<?php
$obj = new OldClass($arg);
"#;
        let config = make_config(&[("OldClass", "NewClass", "make")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("NewClass::make($arg)"));
    }

    #[test]
    fn test_multiple_transformations() {
        let source = r#"<?php
$a = new Cookie($name);
$b = new Response($data);
"#;
        let config = make_config(&[
            ("Cookie", "Cookie", "create"),
            ("Response", "Response", "new"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Cookie::create($name)"));
        assert!(result.contains("Response::new($data)"));
    }

    #[test]
    fn test_case_insensitive() {
        let source = r#"<?php
$a = new COOKIE($name);
$b = new Cookie($name);
"#;
        let config = make_config(&[("Cookie", "Cookie", "create")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$obj = new OtherClass();
"#;
        let config = make_config(&[("Cookie", "Cookie", "create")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$cookie = new Cookie($name);
"#;
        let config = NewToStaticCallConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_from_string_map() {
        let mut map = HashMap::new();
        map.insert("Cookie".to_string(), "Cookie::create".to_string());

        let rule = NewToStaticCallRule::from_string_map(map);
        assert_eq!(rule.config.mappings.len(), 1);
        assert_eq!(rule.config.mappings[0].class_name, "Cookie");
        assert_eq!(rule.config.mappings[0].target_class, "Cookie");
        assert_eq!(rule.config.mappings[0].target_method, "create");
    }

    #[test]
    fn test_in_class() {
        let source = r#"<?php
class Controller {
    public function action() {
        return new Response($data);
    }
}
"#;
        let config = make_config(&[("Response", "Response", "new")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_namespaced_target() {
        let source = r#"<?php
$obj = new Cookie($name);
"#;
        let config = make_config(&[("Cookie", "Http\\Cookie", "create")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Http\\Cookie::create($name)"));
    }
}
