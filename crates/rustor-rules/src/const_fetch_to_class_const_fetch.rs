//! Rule: const_fetch_to_class_const_fetch (Configurable)
//!
//! Transforms global constant fetches to class constant fetches.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.const_fetch_to_class_const_fetch]
//! mappings = [
//!     { old_const = "CONTEXT_COURSE", class = "context\\course", const = "LEVEL" },
//!     { old_const = "SOME_CONST", class = "SomeClass", const = "VALUE" }
//! ]
//! ```
//!
//! Example transformation:
//! ```php
//! // Before
//! $x = CONTEXT_COURSE;
//! $y = SOME_CONST;
//!
//! // After
//! $x = context\course::LEVEL;
//! $y = SomeClass::VALUE;
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// A single constant to class constant mapping
#[derive(Debug, Clone)]
pub struct ConstMapping {
    pub old_const: String,
    pub class_name: String,
    pub const_name: String,
}

/// Configuration for the const_fetch_to_class_const_fetch rule
#[derive(Debug, Clone, Default)]
pub struct ConstFetchToClassConstFetchConfig {
    /// List of mappings from old constants to class::const
    pub mappings: Vec<ConstMapping>,
}

/// Check a parsed PHP program for constant fetches to replace
pub fn check_const_fetch_to_class_const_fetch<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_const_fetch_to_class_const_fetch_with_config(
        program,
        source,
        &ConstFetchToClassConstFetchConfig::default(),
    )
}

/// Check with configuration
pub fn check_const_fetch_to_class_const_fetch_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &ConstFetchToClassConstFetchConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut visitor = ConstFetchToClassConstFetchVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct ConstFetchToClassConstFetchVisitor<'s, 'c> {
    source: &'s str,
    config: &'c ConstFetchToClassConstFetchConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> ConstFetchToClassConstFetchVisitor<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}

impl<'a, 's, 'c> Visitor<'a> for ConstFetchToClassConstFetchVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::ConstantAccess(const_access) = expr {
            self.check_const_access(const_access, expr.span());
        }
        true
    }
}

impl<'s, 'c> ConstFetchToClassConstFetchVisitor<'s, 'c> {
    fn check_const_access(&mut self, const_access: &ConstantAccess<'_>, span: mago_span::Span) {
        let const_name = self.get_text(const_access.name.span());

        // Look up in mappings (case-insensitive for matching)
        let const_name_lower = const_name.to_lowercase();

        let mapping = self.config.mappings.iter().find(|m| {
            m.old_const.to_lowercase() == const_name_lower
        });

        if let Some(mapping) = mapping {
            let replacement = format!("{}::{}", mapping.class_name, mapping.const_name);

            self.edits.push(Edit::new(
                span,
                replacement.clone(),
                format!("Replace {} with {}", const_name, replacement),
            ));
        }
    }
}

/// Rule to transform constant fetches to class constant fetches
pub struct ConstFetchToClassConstFetchRule {
    config: ConstFetchToClassConstFetchConfig,
}

impl ConstFetchToClassConstFetchRule {
    pub fn new() -> Self {
        Self {
            config: ConstFetchToClassConstFetchConfig::default(),
        }
    }

    pub fn with_mappings(mappings: Vec<ConstMapping>) -> Self {
        Self {
            config: ConstFetchToClassConstFetchConfig { mappings },
        }
    }

    /// Convenience constructor using a simple string map format: "OLD_CONST" => "Class::CONST"
    pub fn from_string_map(map: HashMap<String, String>) -> Self {
        let mappings = map
            .into_iter()
            .filter_map(|(old, new)| {
                // Parse "Class::CONST" format
                let parts: Vec<&str> = new.split("::").collect();
                if parts.len() == 2 {
                    Some(ConstMapping {
                        old_const: old,
                        class_name: parts[0].to_string(),
                        const_name: parts[1].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Self {
            config: ConstFetchToClassConstFetchConfig { mappings },
        }
    }
}

impl Default for ConstFetchToClassConstFetchRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for ConstFetchToClassConstFetchRule {
    fn name(&self) -> &'static str {
        "const_fetch_to_class_const_fetch"
    }

    fn description(&self) -> &'static str {
        "Transform global constant fetches to class constant fetches"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_const_fetch_to_class_const_fetch_with_config(program, source, &self.config)
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
            description: "Map of constant names to class::const. Example: { \"OLD_CONST\" = \"Class::CONST\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for ConstFetchToClassConstFetchRule {
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

    fn check_php_with_config(source: &str, config: &ConstFetchToClassConstFetchConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_const_fetch_to_class_const_fetch_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &ConstFetchToClassConstFetchConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str, &str)]) -> ConstFetchToClassConstFetchConfig {
        ConstFetchToClassConstFetchConfig {
            mappings: mappings
                .iter()
                .map(|(old, class, cnst)| ConstMapping {
                    old_const: old.to_string(),
                    class_name: class.to_string(),
                    const_name: cnst.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_simple_const_to_class_const() {
        let source = r#"<?php
$x = CONTEXT_COURSE;
"#;
        let config = make_config(&[("CONTEXT_COURSE", "context\\course", "LEVEL")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("context\\course::LEVEL"));
    }

    #[test]
    fn test_multiple_consts() {
        let source = r#"<?php
$a = CONTEXT_COURSE;
$b = CONTEXT_MODULE;
"#;
        let config = make_config(&[
            ("CONTEXT_COURSE", "context\\course", "LEVEL"),
            ("CONTEXT_MODULE", "context\\module", "LEVEL"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);

        let result = transform_with_config(source, &config);
        assert!(result.contains("context\\course::LEVEL"));
        assert!(result.contains("context\\module::LEVEL"));
    }

    #[test]
    fn test_case_insensitive() {
        let source = r#"<?php
$a = context_course;
$b = CONTEXT_COURSE;
$c = Context_Course;
"#;
        let config = make_config(&[("CONTEXT_COURSE", "Course", "LEVEL")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 3);
    }

    #[test]
    fn test_in_expression() {
        let source = r#"<?php
if ($level === CONTEXT_COURSE) {
    doSomething();
}
"#;
        let config = make_config(&[("CONTEXT_COURSE", "Course", "LEVEL")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("Course::LEVEL"));
    }

    #[test]
    fn test_skip_unmatched() {
        let source = r#"<?php
$x = SOME_OTHER_CONST;
"#;
        let config = make_config(&[("CONTEXT_COURSE", "Course", "LEVEL")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_builtin_consts() {
        // Built-in constants like true, false, null should not match
        // unless explicitly configured
        let source = r#"<?php
$x = true;
$y = false;
$z = null;
"#;
        let config = make_config(&[("CONTEXT_COURSE", "Course", "LEVEL")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$x = CONTEXT_COURSE;
"#;
        let config = ConstFetchToClassConstFetchConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_from_string_map() {
        let mut map = HashMap::new();
        map.insert("OLD_CONST".to_string(), "NewClass::VALUE".to_string());

        let rule = ConstFetchToClassConstFetchRule::from_string_map(map);
        assert_eq!(rule.config.mappings.len(), 1);
        assert_eq!(rule.config.mappings[0].old_const, "OLD_CONST");
        assert_eq!(rule.config.mappings[0].class_name, "NewClass");
        assert_eq!(rule.config.mappings[0].const_name, "VALUE");
    }

    #[test]
    fn test_in_class() {
        let source = r#"<?php
class Foo {
    public function bar() {
        return CONTEXT_COURSE;
    }
}
"#;
        let config = make_config(&[("CONTEXT_COURSE", "Course", "LEVEL")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_in_array() {
        let source = r#"<?php
$arr = [
    CONTEXT_COURSE => 'course',
    CONTEXT_MODULE => 'module',
];
"#;
        let config = make_config(&[
            ("CONTEXT_COURSE", "Course", "LEVEL"),
            ("CONTEXT_MODULE", "Module", "LEVEL"),
        ]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }
}
