//! Rule: rename_attribute (Configurable, PHP 8.0+)
//!
//! Renames PHP 8 attribute class names based on a configurable mapping.
//!
//! Pattern:
//! ```php
//! // Before
//! #[OldAttribute]
//! class SomeClass {}
//!
//! // After
//! #[NewAttribute]
//! class SomeClass {}
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// Configuration for the rename_attribute rule
#[derive(Debug, Clone, Default)]
pub struct RenameAttributeConfig {
    /// Map of old attribute names to new attribute names
    pub mappings: HashMap<String, String>,
}

pub fn check_rename_attribute<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_rename_attribute_with_config(program, source, &RenameAttributeConfig::default())
}

pub fn check_rename_attribute_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &RenameAttributeConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut checker = RenameAttributeChecker {
        source,
        config,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct RenameAttributeChecker<'s, 'c> {
    source: &'s str,
    config: &'c RenameAttributeConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> RenameAttributeChecker<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn find_mapping(&self, attr_name: &str) -> Option<&String> {
        // Try exact match first
        if let Some(new_name) = self.config.mappings.get(attr_name) {
            return Some(new_name);
        }
        // Try case-insensitive match
        let lower = attr_name.to_lowercase();
        self.config.mappings.iter().find_map(|(old, new)| {
            if old.to_lowercase() == lower {
                Some(new)
            } else {
                None
            }
        })
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
                            if let MethodBody::Concrete(ref body) = method.body {
                                self.check_block(body);
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
                self.check_block(&func.body);
            }
            Statement::Namespace(ns) => {
                let statements = match &ns.body {
                    NamespaceBody::Implicit(body) => &body.statements,
                    NamespaceBody::BraceDelimited(body) => &body.statements,
                };
                for inner in statements.iter() {
                    self.check_statement(inner);
                }
            }
            Statement::Block(block) => {
                self.check_block(block);
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &Block<'_>) {
        for stmt in block.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_attribute_list(&mut self, attr_list: &AttributeList<'_>) {
        for attr in attr_list.attributes.nodes.iter() {
            let attr_name = self.get_text(attr.name.span());

            if let Some(new_name) = self.find_mapping(attr_name) {
                self.edits.push(Edit::new(
                    attr.name.span(),
                    new_name.clone(),
                    format!("Rename attribute {} to {}", attr_name, new_name),
                ));
            }
        }
    }
}

pub struct RenameAttributeRule {
    config: RenameAttributeConfig,
}

impl RenameAttributeRule {
    pub fn new() -> Self {
        Self {
            config: RenameAttributeConfig::default(),
        }
    }

    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        Self {
            config: RenameAttributeConfig { mappings },
        }
    }
}

impl Default for RenameAttributeRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RenameAttributeRule {
    fn name(&self) -> &'static str {
        "rename_attribute"
    }

    fn description(&self) -> &'static str {
        "Rename PHP 8 attribute class names based on configurable mapping"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_rename_attribute_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[ConfigOption {
            name: "mappings",
            description: "Map of old attribute names to new attribute names",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for RenameAttributeRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let mappings = config
            .get("mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        Self {
            config: RenameAttributeConfig { mappings },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &RenameAttributeConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_rename_attribute_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &RenameAttributeConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str)]) -> RenameAttributeConfig {
        RenameAttributeConfig {
            mappings: mappings
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn test_class_attribute() {
        let source = r#"<?php
#[OldAttribute]
class SomeClass {}
"#;
        let config = make_config(&[("OldAttribute", "NewAttribute")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("#[NewAttribute]"));
    }

    #[test]
    fn test_method_attribute() {
        let source = r#"<?php
class SomeClass {
    #[OldAttribute]
    public function myMethod() {}
}
"#;
        let config = make_config(&[("OldAttribute", "NewAttribute")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("#[NewAttribute]"));
    }

    #[test]
    fn test_property_attribute() {
        let source = r#"<?php
class SomeClass {
    #[OldAttribute]
    public $property;
}
"#;
        let config = make_config(&[("OldAttribute", "NewAttribute")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_parameter_attribute() {
        let source = r#"<?php
class SomeClass {
    public function myMethod(#[OldAttribute] $param) {}
}
"#;
        let config = make_config(&[("OldAttribute", "NewAttribute")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_multiple_attributes() {
        let source = r#"<?php
#[OldAttribute]
#[AnotherOld]
class SomeClass {}
"#;
        let config = make_config(&[("OldAttribute", "NewAttribute"), ("AnotherOld", "AnotherNew")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_attribute_with_args() {
        let source = r#"<?php
#[OldAttribute('arg1', 'arg2')]
class SomeClass {}
"#;
        let config = make_config(&[("OldAttribute", "NewAttribute")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("#[NewAttribute('arg1', 'arg2')]"));
    }

    #[test]
    fn test_function_attribute() {
        let source = r#"<?php
#[OldAttribute]
function myFunction() {}
"#;
        let config = make_config(&[("OldAttribute", "NewAttribute")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
#[SomeAttribute]
class SomeClass {}
"#;
        let config = make_config(&[("OldAttribute", "NewAttribute")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
#[SomeAttribute]
class SomeClass {}
"#;
        let config = RenameAttributeConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }
}
