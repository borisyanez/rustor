//! Rule: func_call_to_new (Configurable)
//!
//! Transforms function calls to new instance creations.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.func_call_to_new]
//! mappings = { "collection" = "Collection", "response" = "Response" }
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! $items = collection([1, 2, 3]);
//! $resp = response($data, 200);
//!
//! // After
//! $items = new \Collection([1, 2, 3]);
//! $resp = new \Response($data, 200);
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// Configuration for the func_call_to_new rule
#[derive(Debug, Clone, Default)]
pub struct FuncCallToNewConfig {
    /// Map of function names to class names
    pub mappings: HashMap<String, String>,
}

/// Check a parsed PHP program for function calls to replace with new instances
pub fn check_func_call_to_new<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_func_call_to_new_with_config(program, source, &FuncCallToNewConfig::default())
}

/// Check a parsed PHP program for function calls to replace with new instances
pub fn check_func_call_to_new_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &FuncCallToNewConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = FuncCallToNewVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct FuncCallToNewVisitor<'s, 'c> {
    source: &'s str,
    config: &'c FuncCallToNewConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> FuncCallToNewVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's, 'c> Visitor<'a> for FuncCallToNewVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            self.check_function_call(call, expr.span());
        }
        true
    }
}

impl<'s, 'c> FuncCallToNewVisitor<'s, 'c> {
    fn check_function_call(&mut self, call: &FunctionCall<'_>, full_span: mago_span::Span) {
        // Get the function name
        let func_name = match &call.function {
            Expression::Identifier(ident) => self.get_text(ident.span()),
            _ => return, // Skip dynamic/variable function calls
        };

        // Look up in mappings (case-insensitive)
        let func_name_lower = func_name.to_lowercase();

        let class_name = self.config.mappings.iter().find_map(|(old, new)| {
            if old.to_lowercase() == func_name_lower {
                Some(new.clone())
            } else {
                None
            }
        });

        if let Some(class_name) = class_name {
            // Get arguments text
            let args_text = self.get_text(call.argument_list.span());

            // Build new expression: new \ClassName(args)
            let replacement = format!("new \\{}{}", class_name, args_text);

            self.edits.push(Edit::new(
                full_span,
                replacement,
                format!("Replace {}() with new \\{}()", func_name, class_name),
            ));
        }
    }
}

/// Rule to transform function calls to new instances
pub struct FuncCallToNewRule {
    config: FuncCallToNewConfig,
}

impl FuncCallToNewRule {
    pub fn new() -> Self {
        Self {
            config: FuncCallToNewConfig::default(),
        }
    }

    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        Self {
            config: FuncCallToNewConfig { mappings },
        }
    }
}

impl Default for FuncCallToNewRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for FuncCallToNewRule {
    fn name(&self) -> &'static str {
        "func_call_to_new"
    }

    fn description(&self) -> &'static str {
        "Transform function calls to new instance creations"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_func_call_to_new_with_config(program, source, &self.config)
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
            description: "Map of function names to class names. Example: { \"collection\" = \"Collection\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for FuncCallToNewRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let mappings = config
            .get("mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        Self {
            config: FuncCallToNewConfig { mappings },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &FuncCallToNewConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_func_call_to_new_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &FuncCallToNewConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str)]) -> FuncCallToNewConfig {
        FuncCallToNewConfig {
            mappings: mappings
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn test_collection_helper() {
        let source = r#"<?php
$items = collection([1, 2, 3]);
"#;
        let config = make_config(&[("collection", "Collection")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("new \\Collection([1, 2, 3])"));
    }

    #[test]
    fn test_response_helper() {
        let source = r#"<?php
$resp = response($data, 200);
"#;
        let config = make_config(&[("response", "Response")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("new \\Response($data, 200)"));
    }

    #[test]
    fn test_no_args() {
        let source = r#"<?php
$obj = factory();
"#;
        let config = make_config(&[("factory", "Factory")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("new \\Factory()"));
    }

    #[test]
    fn test_multiple_transformations() {
        let source = r#"<?php
$a = collection([]);
$b = response('ok');
"#;
        let config = make_config(&[
            ("collection", "Collection"),
            ("response", "Response"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);

        let result = transform_with_config(source, &config);
        assert!(result.contains("new \\Collection([])"));
        assert!(result.contains("new \\Response('ok')"));
    }

    #[test]
    fn test_fully_qualified_class() {
        let source = r#"<?php
$items = collect([]);
"#;
        let config = make_config(&[("collect", "Illuminate\\Support\\Collection")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("new \\Illuminate\\Support\\Collection([])"));
    }

    #[test]
    fn test_case_insensitive() {
        let source = r#"<?php
$a = COLLECTION([]);
$b = Collection([]);
"#;
        let config = make_config(&[("collection", "Collection")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$result = other_func();
"#;
        let config = make_config(&[("collection", "Collection")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$items = collection([]);
"#;
        let config = FuncCallToNewConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_in_method() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return collection($this->items);
    }
}
"#;
        let config = make_config(&[("collection", "Collection")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_nested_in_return() {
        let source = r#"<?php
return response(collection([]));
"#;
        let config = make_config(&[
            ("collection", "Collection"),
            ("response", "Response"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }
}
