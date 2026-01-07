//! Rule trait and registry for rustor refactoring rules

use mago_syntax::ast::Program;
use rustor_core::Edit;
use std::collections::HashSet;
use std::fmt;
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
                "array_syntax",
                "assign_coalesce",
                "constructor_promotion",
                "first_class_callables",
                "list_short_syntax",
                "isset_coalesce",
                "empty_coalesce",
                "match_expression",
                "null_safe_operator",
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
}

/// Configuration values for rules, keyed by rule name
pub type RuleConfigs = std::collections::HashMap<String, std::collections::HashMap<String, ConfigValue>>;

/// A configuration value that can be passed to rules
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Bool(bool),
    Int(i64),
    String(String),
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
}

impl RuleRegistry {
    /// Create a new registry with all built-in rules using default configuration
    pub fn new() -> Self {
        Self::new_with_config(&RuleConfigs::new())
    }

    /// Create a new registry with all built-in rules using the given configuration
    pub fn new_with_config(configs: &RuleConfigs) -> Self {
        use std::collections::HashMap;

        let mut registry = Self { rules: Vec::new() };

        // Helper to get config for a rule or empty map
        let get_config = |name: &str| -> HashMap<String, ConfigValue> {
            configs.get(name).cloned().unwrap_or_default()
        };

        // Register all built-in rules (configurable rules use their config)
        registry.register(Box::new(super::array_key_first_last::ArrayKeyFirstLastRule));
        registry.register(Box::new(super::array_push::ArrayPushRule));
        registry.register(Box::new(super::array_syntax::ArraySyntaxRule));
        registry.register(Box::new(super::assign_coalesce::AssignCoalesceRule));
        registry.register(Box::new(super::class_constructor::ClassConstructorRule));
        registry.register(Box::new(super::constructor_promotion::ConstructorPromotionRule));
        registry.register(Box::new(super::empty_coalesce::EmptyCoalesceRule));
        registry.register(Box::new(super::first_class_callables::FirstClassCallablesRule));
        registry.register(Box::new(super::implode_order::ImplodeOrderRule));
        registry.register(Box::new(super::is_null::IsNullRule));
        registry.register(Box::new(super::isset_coalesce::IssetCoalesceRule));
        registry.register(Box::new(super::join_to_implode::JoinToImplodeRule));
        registry.register(Box::new(super::list_short_syntax::ListShortSyntaxRule));
        registry.register(Box::new(super::match_expression::MatchExpressionRule));
        registry.register(Box::new(super::null_safe_operator::NullSafeOperatorRule));
        registry.register(Box::new(super::pow_to_operator::PowToOperatorRule));
        registry.register(Box::new(super::readonly_properties::ReadonlyPropertiesRule));
        registry.register(Box::new(super::sizeof::SizeofRule));
        registry.register(Box::new(super::sprintf_positional::SprintfPositionalRule));
        registry.register(Box::new(super::string_contains::StringContainsRule::with_config(&get_config("string_contains"))));
        registry.register(Box::new(super::string_starts_ends::StringStartsEndsRule));
        registry.register(Box::new(super::type_cast::TypeCastRule));

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
