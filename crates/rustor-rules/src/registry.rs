//! Rule trait and registry for rustor refactoring rules

use mago_syntax::ast::Program;
use rustor_core::Edit;
use std::collections::HashSet;

/// A refactoring rule that can detect and suggest code transformations
pub trait Rule: Send + Sync {
    /// The unique identifier for this rule (e.g., "array_push")
    fn name(&self) -> &'static str;

    /// A short description of what this rule does
    fn description(&self) -> &'static str;

    /// Check a PHP program and return suggested edits
    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit>;
}

/// Registry of all available refactoring rules
pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
}

impl RuleRegistry {
    /// Create a new registry with all built-in rules
    pub fn new() -> Self {
        let mut registry = Self { rules: Vec::new() };

        // Register all built-in rules
        registry.register(Box::new(super::array_push::ArrayPushRule));
        registry.register(Box::new(super::array_syntax::ArraySyntaxRule));
        registry.register(Box::new(super::empty_coalesce::EmptyCoalesceRule));
        registry.register(Box::new(super::is_null::IsNullRule));
        registry.register(Box::new(super::isset_coalesce::IssetCoalesceRule));
        registry.register(Box::new(super::join_to_implode::JoinToImplodeRule));
        registry.register(Box::new(super::list_short_syntax::ListShortSyntaxRule));
        registry.register(Box::new(super::pow_to_operator::PowToOperatorRule));
        registry.register(Box::new(super::sizeof::SizeofRule));
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
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
