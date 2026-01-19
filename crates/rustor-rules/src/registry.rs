//! Rule trait and registry for rustor refactoring rules

use mago_syntax::ast::Program;
use rustor_core::Edit;
use std::collections::HashSet;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

/// PHP version for filtering rules by minimum requirement
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhpVersion {
    Php54,
    Php55,
    Php56,
    Php70,
    Php71,
    Php72,
    Php73,
    Php74,
    Php80,
    Php81,
    Php82,
    Php83,
    Php84,
}

impl PhpVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            PhpVersion::Php54 => "5.4",
            PhpVersion::Php55 => "5.5",
            PhpVersion::Php56 => "5.6",
            PhpVersion::Php70 => "7.0",
            PhpVersion::Php71 => "7.1",
            PhpVersion::Php72 => "7.2",
            PhpVersion::Php73 => "7.3",
            PhpVersion::Php74 => "7.4",
            PhpVersion::Php80 => "8.0",
            PhpVersion::Php81 => "8.1",
            PhpVersion::Php82 => "8.2",
            PhpVersion::Php83 => "8.3",
            PhpVersion::Php84 => "8.4",
        }
    }
}

impl fmt::Display for PhpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for PhpVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "5.4" => Ok(PhpVersion::Php54),
            "5.5" => Ok(PhpVersion::Php55),
            "5.6" => Ok(PhpVersion::Php56),
            "7.0" => Ok(PhpVersion::Php70),
            "7.1" => Ok(PhpVersion::Php71),
            "7.2" => Ok(PhpVersion::Php72),
            "7.3" => Ok(PhpVersion::Php73),
            "7.4" => Ok(PhpVersion::Php74),
            "8.0" => Ok(PhpVersion::Php80),
            "8.1" => Ok(PhpVersion::Php81),
            "8.2" => Ok(PhpVersion::Php82),
            "8.3" => Ok(PhpVersion::Php83),
            "8.4" => Ok(PhpVersion::Php84),
            _ => Err(format!("Invalid PHP version: {}. Valid versions: 5.4, 5.5, 5.6, 7.0-7.4, 8.0-8.4", s)),
        }
    }
}

/// Rule category for organization and filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Rules that improve runtime performance
    Performance,
    /// Rules that modernize syntax to newer PHP features
    Modernization,
    /// Rules that simplify code (fewer function calls, cleaner expressions)
    Simplification,
    /// Rules that ensure compatibility or follow best practices
    Compatibility,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Performance => "performance",
            Category::Modernization => "modernization",
            Category::Simplification => "simplification",
            Category::Compatibility => "compatibility",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Category::Performance => "Rules that improve runtime performance",
            Category::Modernization => "Rules that modernize syntax to newer PHP features",
            Category::Simplification => "Rules that simplify code",
            Category::Compatibility => "Rules that ensure compatibility or follow best practices",
        }
    }

    pub fn all() -> &'static [Category] {
        &[
            Category::Performance,
            Category::Modernization,
            Category::Simplification,
            Category::Compatibility,
        ]
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Category {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "performance" => Ok(Category::Performance),
            "modernization" => Ok(Category::Modernization),
            "simplification" => Ok(Category::Simplification),
            "compatibility" => Ok(Category::Compatibility),
            _ => Err(format!(
                "Invalid category: {}. Valid categories: performance, modernization, simplification, compatibility",
                s
            )),
        }
    }
}

/// Preset rule configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Safe, widely-applicable rules
    Recommended,
    /// Performance-focused rules
    Performance,
    /// Syntax modernization rules
    Modernize,
    /// All available rules
    All,
}

impl Preset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Preset::Recommended => "recommended",
            Preset::Performance => "performance",
            Preset::Modernize => "modernize",
            Preset::All => "all",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Preset::Recommended => "Safe, widely-applicable rules",
            Preset::Performance => "Performance-focused rules",
            Preset::Modernize => "Syntax modernization rules",
            Preset::All => "All available rules",
        }
    }

    pub fn rules(&self) -> &'static [&'static str] {
        match self {
            Preset::Recommended => &[
                "array_push",
                "array_syntax",
                "implode_order",
                "is_null",
                "isset_coalesce",
                "sizeof",
            ],
            Preset::Performance => &[
                "array_key_first_last",
                "array_push",
                "pow_to_operator",
                "sizeof",
                "type_cast",
            ],
            Preset::Modernize => &[
                "arrow_functions",
                "array_syntax",
                "assign_coalesce",
                "constructor_promotion",
                "first_class_callables",
                "get_class_this",
                "get_debug_type",
                "list_short_syntax",
                "isset_coalesce",
                "empty_coalesce",
                "match_expression",
                "null_safe_operator",
                "override_attribute",
                "readonly_properties",
                "string_contains",
                "string_starts_ends",
            ],
            Preset::All => &[], // Special case: all rules
        }
    }

    pub fn all() -> &'static [Preset] {
        &[
            Preset::Recommended,
            Preset::Performance,
            Preset::Modernize,
            Preset::All,
        ]
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Preset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "recommended" => Ok(Preset::Recommended),
            "performance" => Ok(Preset::Performance),
            "modernize" => Ok(Preset::Modernize),
            "all" => Ok(Preset::All),
            _ => Err(format!(
                "Invalid preset: {}. Valid presets: recommended, performance, modernize, all",
                s
            )),
        }
    }
}

/// A refactoring rule that can detect and suggest code transformations
pub trait Rule: Send + Sync {
    /// The unique identifier for this rule (e.g., "array_push")
    fn name(&self) -> &'static str;

    /// A short description of what this rule does
    fn description(&self) -> &'static str;

    /// Check a PHP program and return suggested edits
    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit>;

    /// Minimum PHP version required for the transformed code
    /// Returns None if the rule works on any PHP version
    fn min_php_version(&self) -> Option<PhpVersion> {
        None
    }

    /// The category this rule belongs to
    fn category(&self) -> Category {
        Category::Simplification
    }

    /// Get the list of configurable options for this rule
    fn config_options(&self) -> &'static [ConfigOption] {
        &[]
    }
}

/// Description of a configurable option for a rule
#[derive(Debug, Clone)]
pub struct ConfigOption {
    /// Option name (e.g., "strict_comparison")
    pub name: &'static str,
    /// Description of what this option does
    pub description: &'static str,
    /// Default value as a string representation
    pub default: &'static str,
    /// Type of the option (bool, int, string)
    pub option_type: ConfigOptionType,
}

/// Type of a configuration option
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigOptionType {
    Bool,
    Int,
    String,
    /// A map of string keys to string values (e.g., for rename mappings)
    StringMap,
}

/// Configuration values for rules, keyed by rule name
pub type RuleConfigs = std::collections::HashMap<String, std::collections::HashMap<String, ConfigValue>>;

/// A configuration value that can be passed to rules
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Bool(bool),
    Int(i64),
    String(String),
    /// A map of string keys to string values (e.g., for rename mappings)
    StringMap(std::collections::HashMap<String, String>),
}

impl ConfigValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            ConfigValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_string_map(&self) -> Option<&std::collections::HashMap<String, String>> {
        match self {
            ConfigValue::StringMap(m) => Some(m),
            _ => None,
        }
    }
}

/// Trait for rules that can be configured with runtime options
pub trait ConfigurableRule: Rule {
    /// Create a new instance with the given configuration
    fn with_config(config: &std::collections::HashMap<String, ConfigValue>) -> Self
    where
        Self: Sized;
}

/// Registry of all available refactoring rules
pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
    yaml_rule_dirs: Vec<std::path::PathBuf>,
}

impl RuleRegistry {
    /// Create a new registry with all built-in rules using default configuration
    pub fn new() -> Self {
        Self::new_with_config(&RuleConfigs::new())
    }

    /// Create a new registry with all built-in rules using the given configuration
    pub fn new_with_config(configs: &RuleConfigs) -> Self {
        use std::collections::HashMap;

        let mut registry = Self {
            rules: Vec::new(),
            yaml_rule_dirs: Vec::new(),
        };

        // Helper to get config for a rule or empty map
        let get_config = |name: &str| -> HashMap<String, ConfigValue> {
            configs.get(name).cloned().unwrap_or_default()
        };

        // Register all built-in rules (configurable rules use their config)
        registry.register(Box::new(super::array_key_first_last::ArrayKeyFirstLastRule));
        registry.register(Box::new(super::array_push::ArrayPushRule));
        registry.register(Box::new(super::array_syntax::ArraySyntaxRule));
        registry.register(Box::new(super::arrow_functions::ArrowFunctionsRule));
        registry.register(Box::new(super::assign_coalesce::AssignCoalesceRule));
        registry.register(Box::new(super::class_constructor::ClassConstructorRule));
        registry.register(Box::new(super::constructor_promotion::ConstructorPromotionRule));
        registry.register(Box::new(super::empty_coalesce::EmptyCoalesceRule));
        registry.register(Box::new(super::first_class_callables::FirstClassCallablesRule));
        registry.register(Box::new(super::get_class_this::GetClassThisRule));
        registry.register(Box::new(super::get_debug_type::GetDebugTypeRule));
        registry.register(Box::new(super::implode_order::ImplodeOrderRule));
        registry.register(Box::new(super::is_null::IsNullRule));
        registry.register(Box::new(super::isset_coalesce::IssetCoalesceRule));
        registry.register(Box::new(super::join_to_implode::JoinToImplodeRule));
        registry.register(Box::new(super::list_short_syntax::ListShortSyntaxRule));
        registry.register(Box::new(super::match_expression::MatchExpressionRule));
        registry.register(Box::new(super::null_safe_operator::NullSafeOperatorRule));
        registry.register(Box::new(super::override_attribute::OverrideAttributeRule));
        registry.register(Box::new(super::pow_to_operator::PowToOperatorRule));
        registry.register(Box::new(super::readonly_properties::ReadonlyPropertiesRule));
        registry.register(Box::new(super::redundant_type_check::RedundantTypeCheckRule));
        registry.register(Box::new(super::rename_class::RenameClassRule::with_config(&get_config("rename_class"))));
        registry.register(Box::new(super::rename_function::RenameFunctionRule::with_config(&get_config("rename_function"))));
        registry.register(Box::new(super::single_in_array_to_compare::SingleInArrayToCompareRule));
        registry.register(Box::new(super::sizeof::SizeofRule));
        registry.register(Box::new(super::switch_negated_ternary::SwitchNegatedTernaryRule));
        registry.register(Box::new(super::sprintf_positional::SprintfPositionalRule));
        registry.register(Box::new(super::string_contains::StringContainsRule::with_config(&get_config("string_contains"))));
        registry.register(Box::new(super::string_starts_ends::StringStartsEndsRule));
        registry.register(Box::new(super::type_cast::TypeCastRule));
        registry.register(Box::new(super::utf8_decode_encode::Utf8DecodeEncodeRule));

        // Register imported rules from Rector
        for rule in super::imported::imported_rules() {
            registry.register(rule);
        }

        registry
    }

    /// Register a new rule
    pub fn register(&mut self, rule: Box<dyn Rule>) {
        self.rules.push(rule);
    }

    /// Get all rule names
    pub fn all_names(&self) -> Vec<&'static str> {
        self.rules.iter().map(|r| r.name()).collect()
    }

    /// Get rules filtered by enabled names
    pub fn get_enabled(&self, enabled: &HashSet<String>) -> Vec<&dyn Rule> {
        self.rules
            .iter()
            .filter(|r| enabled.contains(r.name()))
            .map(|r| r.as_ref())
            .collect()
    }

    /// Get all rules with their descriptions (for --list-rules)
    pub fn list_rules(&self) -> Vec<(&'static str, &'static str)> {
        self.rules
            .iter()
            .map(|r| (r.name(), r.description()))
            .collect()
    }

    /// Get all rules with full metadata
    pub fn list_rules_full(&self) -> Vec<RuleInfo> {
        self.rules
            .iter()
            .map(|r| RuleInfo {
                name: r.name(),
                description: r.description(),
                category: r.category(),
                min_php_version: r.min_php_version(),
            })
            .collect()
    }

    /// Get rules for a preset
    pub fn get_preset_rules(&self, preset: Preset) -> HashSet<String> {
        if preset == Preset::All {
            self.all_names().into_iter().map(String::from).collect()
        } else {
            preset.rules().iter().map(|s| String::from(*s)).collect()
        }
    }

    /// Filter rules by category
    pub fn filter_by_category(&self, category: Category) -> Vec<&dyn Rule> {
        self.rules
            .iter()
            .filter(|r| r.category() == category)
            .map(|r| r.as_ref())
            .collect()
    }

    /// Filter rules by minimum PHP version (rules that work on the given version or older)
    pub fn filter_by_php_version(&self, target_version: PhpVersion) -> Vec<&dyn Rule> {
        self.rules
            .iter()
            .filter(|r| {
                r.min_php_version()
                    .map(|v| v <= target_version)
                    .unwrap_or(true)
            })
            .map(|r| r.as_ref())
            .collect()
    }

    /// Get rules filtered by enabled names and optionally by PHP version
    pub fn get_enabled_filtered(
        &self,
        enabled: &HashSet<String>,
        php_version: Option<PhpVersion>,
        category: Option<Category>,
    ) -> Vec<&dyn Rule> {
        self.rules
            .iter()
            .filter(|r| enabled.contains(r.name()))
            .filter(|r| {
                php_version
                    .map(|v| r.min_php_version().map(|rv| rv <= v).unwrap_or(true))
                    .unwrap_or(true)
            })
            .filter(|r| category.map(|c| r.category() == c).unwrap_or(true))
            .map(|r| r.as_ref())
            .collect()
    }

    /// Run all enabled rules on a program
    pub fn check_all<'a>(
        &self,
        program: &Program<'a>,
        source: &str,
        enabled: &HashSet<String>,
    ) -> Vec<Edit> {
        let mut edits = Vec::new();
        for rule in self.get_enabled(enabled) {
            edits.extend(rule.check(program, source));
        }
        edits
    }

    /// Run filtered rules on a program
    pub fn check_filtered<'a>(
        &self,
        program: &Program<'a>,
        source: &str,
        enabled: &HashSet<String>,
        php_version: Option<PhpVersion>,
        category: Option<Category>,
    ) -> Vec<Edit> {
        let mut edits = Vec::new();
        for rule in self.get_enabled_filtered(enabled, php_version, category) {
            edits.extend(rule.check(program, source));
        }
        edits
    }

    /// Load YAML rules from a directory and add them to the registry
    ///
    /// Returns the number of rules loaded, or an error if loading failed.
    pub fn load_yaml_rules_from_dir(&mut self, dir: &Path) -> Result<usize, String> {
        use super::yaml_rules::load_rules_from_dir;

        match load_rules_from_dir(dir) {
            Ok(interpreters) => {
                let count = interpreters.len();
                for interpreter in interpreters {
                    self.register(Box::new(interpreter));
                }
                self.yaml_rule_dirs.push(dir.to_path_buf());
                Ok(count)
            }
            Err(e) => Err(format!("Failed to load YAML rules from {}: {}", dir.display(), e)),
        }
    }

    /// Load YAML rules from a single file and add them to the registry
    pub fn load_yaml_rules_from_file(&mut self, path: &Path) -> Result<usize, String> {
        use super::yaml_rules::load_rules_from_file;

        match load_rules_from_file(path) {
            Ok(interpreters) => {
                let count = interpreters.len();
                for interpreter in interpreters {
                    self.register(Box::new(interpreter));
                }
                Ok(count)
            }
            Err(e) => Err(format!("Failed to load YAML rules from {}: {}", path.display(), e)),
        }
    }

    /// Load YAML rules from a string and add them to the registry
    pub fn load_yaml_rules_from_string(&mut self, yaml: &str) -> Result<usize, String> {
        use super::yaml_rules::load_rules_from_string;

        match load_rules_from_string(yaml) {
            Ok(interpreters) => {
                let count = interpreters.len();
                for interpreter in interpreters {
                    self.register(Box::new(interpreter));
                }
                Ok(count)
            }
            Err(e) => Err(format!("Failed to parse YAML rules: {}", e)),
        }
    }

    /// Load bundled YAML rules from the crate's rules directory
    ///
    /// This loads rules shipped with rustor-rules.
    pub fn load_bundled_yaml_rules(&mut self) -> Result<usize, String> {
        // Try to find the rules directory relative to the manifest
        let rules_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("rules");

        if rules_dir.is_dir() {
            self.load_yaml_rules_from_dir(&rules_dir)
        } else {
            // Rules directory not found - this is OK in release builds
            Ok(0)
        }
    }

    /// Get the list of YAML rule directories that have been loaded
    pub fn yaml_rule_dirs(&self) -> &[std::path::PathBuf] {
        &self.yaml_rule_dirs
    }

    /// Get the total number of rules in the registry
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// Rule information for display
#[derive(Debug, Clone)]
pub struct RuleInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub category: Category,
    pub min_php_version: Option<PhpVersion>,
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_load_bundled_yaml_rules() {
        let mut registry = RuleRegistry::new();
        let initial_count = registry.rule_count();

        let loaded = registry.load_bundled_yaml_rules().expect("Failed to load bundled YAML rules");

        // We have 5 bundled YAML rules
        assert_eq!(loaded, 5, "Expected 5 bundled YAML rules");
        assert_eq!(registry.rule_count(), initial_count + 5);

        // Check that the rules are accessible by name
        let all_names = registry.all_names();
        assert!(all_names.contains(&"is_null_to_comparison"));
        assert!(all_names.contains(&"sizeof_to_count"));
        assert!(all_names.contains(&"strpos_to_str_contains"));
        assert!(all_names.contains(&"array_push_to_assignment"));
        assert!(all_names.contains(&"yaml_join_to_implode"));
    }

    #[test]
    fn test_registry_load_yaml_from_string() {
        let mut registry = RuleRegistry::new();
        let initial_count = registry.rule_count();

        let yaml = r#"
name: test_yaml_rule
description: "A test rule loaded from string"
category: code_quality

match:
  node: FuncCall
  name: old_test_func
  args:
    - capture: $arg

replace: "new_test_func($arg)"

tests:
  - input: "old_test_func($x)"
    output: "new_test_func($x)"
"#;

        let loaded = registry.load_yaml_rules_from_string(yaml).expect("Failed to load YAML rule from string");

        assert_eq!(loaded, 1);
        assert_eq!(registry.rule_count(), initial_count + 1);
        assert!(registry.all_names().contains(&"test_yaml_rule"));
    }

    #[test]
    fn test_yaml_rule_check_integration() {
        use bumpalo::Bump;
        use mago_database::file::FileId;
        use mago_syntax::parser::parse_file_content;

        let mut registry = RuleRegistry::new();
        registry.load_bundled_yaml_rules().unwrap();

        // Test that sizeof_to_count rule works through the registry
        let code = "<?php sizeof($arr);";
        let bump = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = parse_file_content(&bump, file_id, code);

        let mut enabled = HashSet::new();
        enabled.insert("sizeof_to_count".to_string());

        let edits = registry.check_all(program, code, &enabled);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "count($arr)");
    }

    #[test]
    fn test_yaml_rule_join_to_implode() {
        use bumpalo::Bump;
        use mago_database::file::FileId;
        use mago_syntax::parser::parse_file_content;

        let mut registry = RuleRegistry::new();
        registry.load_bundled_yaml_rules().unwrap();

        // Test that yaml_join_to_implode rule works with spread capture
        let code = "<?php join(',', $arr);";
        let bump = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = parse_file_content(&bump, file_id, code);

        let mut enabled = HashSet::new();
        enabled.insert("yaml_join_to_implode".to_string());

        let edits = registry.check_all(program, code, &enabled);

        assert_eq!(edits.len(), 1, "Should find one edit");
        assert_eq!(edits[0].replacement, "implode(',', $arr)", "Should replace with implode and preserve args");
    }
}
