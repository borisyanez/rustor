//! Rule: attribute_key_to_class_const_fetch (Configurable)
//!
//! Transforms attribute argument string values to class constants.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.attribute_key_to_class_const_fetch]
//! mappings = [
//!   { attribute = "Column", key = "type", class = "Types", values = { "string" = "STRING", "integer" = "INTEGER" } }
//! ]
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! #[Column(type: "string")]
//! public $name;
//!
//! // After
//! #[Column(type: Types::STRING)]
//! public $name;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// A single attribute key to class constant mapping
#[derive(Debug, Clone)]
pub struct AttributeKeyMapping {
    /// The attribute class name (short name like "Column" or FQN)
    pub attribute_class: String,
    /// The argument key name
    pub argument_key: String,
    /// The target class for constants
    pub target_class: String,
    /// Map of string values to constant names
    pub value_to_const: HashMap<String, String>,
}

/// Configuration for the attribute_key_to_class_const_fetch rule
#[derive(Debug, Clone, Default)]
pub struct AttributeKeyToClassConstFetchConfig {
    pub mappings: Vec<AttributeKeyMapping>,
}

pub fn check_attribute_key_to_class_const_fetch<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_attribute_key_to_class_const_fetch_with_config(program, source, &AttributeKeyToClassConstFetchConfig::default())
}

pub fn check_attribute_key_to_class_const_fetch_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &AttributeKeyToClassConstFetchConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut checker = AttributeKeyChecker {
        source,
        config,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct AttributeKeyChecker<'s, 'c> {
    source: &'s str,
    config: &'c AttributeKeyToClassConstFetchConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> AttributeKeyChecker<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Extract the string value from a string literal (removes quotes)
    fn extract_string_value(&self, span: mago_span::Span) -> Option<String> {
        let full_text = self.get_text(span);

        let quote_char = full_text.chars().next()?;
        if quote_char != '\'' && quote_char != '"' {
            return None;
        }

        if full_text.len() < 2 {
            return None;
        }

        Some(full_text[1..full_text.len() - 1].to_string())
    }

    /// Get the short name from a potentially namespaced identifier
    fn get_short_name<'a>(&self, name: &'a str) -> &'a str {
        name.rsplit('\\').next().unwrap_or(name)
    }

    fn check_program(&mut self, program: &Program<'_>) {
        for stmt in program.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::Class(class) => {
                // Check class attributes
                for attr_list in class.attribute_lists.iter() {
                    self.check_attribute_list(attr_list);
                }
                // Check members
                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Method(method) => {
                            for attr_list in method.attribute_lists.iter() {
                                self.check_attribute_list(attr_list);
                            }
                            // Check parameter attributes
                            for param in method.parameter_list.parameters.iter() {
                                for attr_list in param.attribute_lists.iter() {
                                    self.check_attribute_list(attr_list);
                                }
                            }
                        }
                        ClassLikeMember::Property(Property::Plain(prop)) => {
                            for attr_list in prop.attribute_lists.iter() {
                                self.check_attribute_list(attr_list);
                            }
                        }
                        ClassLikeMember::Constant(const_member) => {
                            for attr_list in const_member.attribute_lists.iter() {
                                self.check_attribute_list(attr_list);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::Interface(iface) => {
                for attr_list in iface.attribute_lists.iter() {
                    self.check_attribute_list(attr_list);
                }
            }
            Statement::Trait(trait_def) => {
                for attr_list in trait_def.attribute_lists.iter() {
                    self.check_attribute_list(attr_list);
                }
            }
            Statement::Enum(enum_def) => {
                for attr_list in enum_def.attribute_lists.iter() {
                    self.check_attribute_list(attr_list);
                }
            }
            Statement::Function(func) => {
                for attr_list in func.attribute_lists.iter() {
                    self.check_attribute_list(attr_list);
                }
                // Check parameter attributes
                for param in func.parameter_list.parameters.iter() {
                    for attr_list in param.attribute_lists.iter() {
                        self.check_attribute_list(attr_list);
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner);
                }
            }
            Statement::Namespace(ns) => {
                for inner in ns.statements().iter() {
                    self.check_statement(inner);
                }
            }
            _ => {}
        }
    }

    fn check_attribute_list(&mut self, attr_list: &AttributeList<'_>) {
        for attribute in attr_list.attributes.nodes.iter() {
            self.check_attribute(attribute);
        }
    }

    fn check_attribute(&mut self, attribute: &Attribute<'_>) {
        // Get the attribute name and convert to owned string to avoid borrow issues
        let attr_name = self.get_text(attribute.name.span()).to_string();
        let attr_short_name = self.get_short_name(&attr_name).to_lowercase();

        // Find matching mappings for this attribute
        for mapping in &self.config.mappings {
            let mapping_short_name = self.get_short_name(&mapping.attribute_class).to_lowercase();

            // Case-insensitive comparison for class names
            if attr_short_name != mapping_short_name {
                continue;
            }

            // Check the argument list (if present)
            if let Some(ref args) = attribute.argument_list {
                self.check_arguments(args, mapping);
            }
        }
    }

    fn check_arguments(&mut self, args: &ArgumentList<'_>, mapping: &AttributeKeyMapping) {
        for arg in args.arguments.iter() {
            // We only care about named arguments
            if let Argument::Named(named_arg) = arg {
                let arg_name = self.get_text(named_arg.name.span());

                // Check if this is the key we're looking for
                if arg_name == mapping.argument_key {
                    self.check_argument_value(&named_arg.value, mapping);
                }
            }
        }
    }

    fn check_argument_value(&mut self, value: &Expression<'_>, mapping: &AttributeKeyMapping) {
        // Only process string literals
        if let Expression::Literal(Literal::String(string_lit)) = value {
            let string_value = match self.extract_string_value(string_lit.span()) {
                Some(v) => v,
                None => return,
            };

            // Look up the constant name
            if let Some(const_name) = mapping.value_to_const.get(&string_value) {
                let replacement = format!("{}::{}", mapping.target_class, const_name);

                self.edits.push(Edit::new(
                    string_lit.span(),
                    replacement.clone(),
                    format!("Replace \"{}\" with {}", string_value, replacement),
                ));
            }
        }
    }
}

pub struct AttributeKeyToClassConstFetchRule {
    config: AttributeKeyToClassConstFetchConfig,
}

impl AttributeKeyToClassConstFetchRule {
    pub fn new() -> Self {
        Self { config: AttributeKeyToClassConstFetchConfig::default() }
    }

    pub fn with_mappings(mappings: Vec<AttributeKeyMapping>) -> Self {
        Self { config: AttributeKeyToClassConstFetchConfig { mappings } }
    }
}

impl Default for AttributeKeyToClassConstFetchRule {
    fn default() -> Self { Self::new() }
}

impl Rule for AttributeKeyToClassConstFetchRule {
    fn name(&self) -> &'static str { "attribute_key_to_class_const_fetch" }
    fn description(&self) -> &'static str { "Transform attribute argument values to class constants" }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_attribute_key_to_class_const_fetch_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category { Category::Modernization }
    fn min_php_version(&self) -> Option<PhpVersion> { Some(PhpVersion::Php80) }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[ConfigOption {
            name: "mappings",
            description: "List of attribute key to constant mappings. Complex config - see documentation.",
            default: "[]",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for AttributeKeyToClassConstFetchRule {
    fn with_config(_config: &HashMap<String, ConfigValue>) -> Self {
        // Note: This rule has complex config that's hard to express in simple TOML
        // For now, return default. Full config parsing would need custom TOML structure.
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &AttributeKeyToClassConstFetchConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_attribute_key_to_class_const_fetch_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &AttributeKeyToClassConstFetchConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_column_config() -> AttributeKeyToClassConstFetchConfig {
        let mut value_map = HashMap::new();
        value_map.insert("string".to_string(), "STRING".to_string());
        value_map.insert("integer".to_string(), "INTEGER".to_string());
        value_map.insert("boolean".to_string(), "BOOLEAN".to_string());

        AttributeKeyToClassConstFetchConfig {
            mappings: vec![AttributeKeyMapping {
                attribute_class: "Column".to_string(),
                argument_key: "type".to_string(),
                target_class: "Types".to_string(),
                value_to_const: value_map,
            }],
        }
    }

    #[test]
    fn test_column_type_string() {
        let source = r#"<?php
class Entity {
    #[Column(type: "string")]
    public $name;
}
"#;
        let config = make_column_config();
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("Types::STRING"));
    }

    #[test]
    fn test_column_type_integer() {
        let source = r#"<?php
class Entity {
    #[Column(type: "integer")]
    public $count;
}
"#;
        let config = make_column_config();
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("Types::INTEGER"));
    }

    #[test]
    fn test_single_quoted_string() {
        let source = r#"<?php
class Entity {
    #[Column(type: 'boolean')]
    public $active;
}
"#;
        let config = make_column_config();
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("Types::BOOLEAN"));
    }

    #[test]
    fn test_multiple_attributes() {
        let source = r#"<?php
class Entity {
    #[Column(type: "string")]
    public $name;

    #[Column(type: "integer")]
    public $age;
}
"#;
        let config = make_column_config();
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_other_keys() {
        let source = r#"<?php
class Entity {
    #[Column(name: "string", length: 255)]
    public $name;
}
"#;
        let config = make_column_config();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_other_attributes() {
        let source = r#"<?php
class Entity {
    #[Entity(type: "string")]
    public $name;
}
"#;
        let config = make_column_config();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_unmapped_value() {
        let source = r#"<?php
class Entity {
    #[Column(type: "text")]
    public $description;
}
"#;
        let config = make_column_config();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
class Entity {
    #[Column(type: "string")]
    public $name;
}
"#;
        let config = AttributeKeyToClassConstFetchConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_fqn_attribute() {
        let source = r#"<?php
class Entity {
    #[\Doctrine\ORM\Mapping\Column(type: "string")]
    public $name;
}
"#;
        let mut value_map = HashMap::new();
        value_map.insert("string".to_string(), "STRING".to_string());

        let config = AttributeKeyToClassConstFetchConfig {
            mappings: vec![AttributeKeyMapping {
                attribute_class: "Doctrine\\ORM\\Mapping\\Column".to_string(),
                argument_key: "type".to_string(),
                target_class: "Types".to_string(),
                value_to_const: value_map,
            }],
        };
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_mixed_args_with_type() {
        let source = r#"<?php
class Entity {
    #[Column(name: "user_name", type: "string", length: 100)]
    public $name;
}
"#;
        let config = make_column_config();
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("Types::STRING"));
    }
}
