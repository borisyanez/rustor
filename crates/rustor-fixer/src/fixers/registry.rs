//! Fixer registry for managing available fixers
//!
//! The registry collects all available fixers and provides methods
//! to look them up by name and apply them in priority order.

use std::collections::HashMap;
use std::sync::Arc;
use rustor_core::Edit;

use super::{Fixer, FixerConfig};
use super::whitespace::{
    TrailingWhitespaceFixer,
    LineEndingFixer,
    IndentationFixer,
    SingleBlankLineAtEofFixer,
    NoWhitespaceInBlankLineFixer,
    EncodingFixer,
    FullOpeningTagFixer,
    BlankLineAfterOpeningTagFixer,
};
use super::casing::{
    LowercaseKeywordsFixer,
    ConstantCaseFixer,
    LowercaseStaticReferenceFixer,
};
use super::braces::{
    ElseifFixer,
    NoClosingTagFixer,
    SwitchCaseSpaceFixer,
    BracesPositionFixer,
};
use super::functions::{
    FunctionDeclarationFixer,
    MethodArgumentSpaceFixer,
    ReturnTypeDeclarationFixer,
};
use super::operators::{
    ConcatSpaceFixer,
    BinaryOperatorSpacesFixer,
    UnaryOperatorSpacesFixer,
};
use super::imports::{
    BlankLineAfterNamespaceFixer,
    NoLeadingImportSlashFixer,
    SingleLineAfterImportsFixer,
    OrderedImportsFixer,
    SingleImportPerStatementFixer,
    NoUnusedImportsFixer,
};
use super::comments::{
    NoTrailingWhitespaceInCommentFixer,
    SingleLineCommentStyleFixer,
    MultilineWhitespaceBeforeSemicolonsFixer,
};
use super::class::{
    VisibilityRequiredFixer,
    NoBlankLinesAfterClassOpeningFixer,
    ClassDefinitionFixer,
};

/// Information about a registered fixer
#[derive(Clone)]
pub struct FixerInfo {
    pub name: &'static str,
    pub php_cs_fixer_name: &'static str,
    pub description: &'static str,
    pub priority: i32,
    pub is_risky: bool,
}

/// Registry of all available fixers
pub struct FixerRegistry {
    fixers: Vec<Arc<dyn Fixer>>,
    by_name: HashMap<&'static str, usize>,
    by_php_name: HashMap<&'static str, usize>,
}

impl FixerRegistry {
    /// Create a new registry with all built-in fixers
    pub fn new() -> Self {
        let mut registry = Self {
            fixers: Vec::new(),
            by_name: HashMap::new(),
            by_php_name: HashMap::new(),
        };

        // Register whitespace fixers
        registry.register(Arc::new(EncodingFixer));
        registry.register(Arc::new(FullOpeningTagFixer));
        registry.register(Arc::new(BlankLineAfterOpeningTagFixer));
        registry.register(Arc::new(LineEndingFixer));
        registry.register(Arc::new(TrailingWhitespaceFixer));
        registry.register(Arc::new(NoWhitespaceInBlankLineFixer));
        registry.register(Arc::new(IndentationFixer));
        registry.register(Arc::new(SingleBlankLineAtEofFixer));

        // Register casing fixers
        registry.register(Arc::new(LowercaseKeywordsFixer));
        registry.register(Arc::new(ConstantCaseFixer));
        registry.register(Arc::new(LowercaseStaticReferenceFixer));

        // Register braces/control structure fixers
        registry.register(Arc::new(NoClosingTagFixer));
        registry.register(Arc::new(ElseifFixer));
        registry.register(Arc::new(SwitchCaseSpaceFixer));
        registry.register(Arc::new(BracesPositionFixer));

        // Register function fixers
        registry.register(Arc::new(FunctionDeclarationFixer));
        registry.register(Arc::new(MethodArgumentSpaceFixer));
        registry.register(Arc::new(ReturnTypeDeclarationFixer));

        // Register operator fixers
        registry.register(Arc::new(ConcatSpaceFixer));
        registry.register(Arc::new(BinaryOperatorSpacesFixer));
        registry.register(Arc::new(UnaryOperatorSpacesFixer));

        // Register import/namespace fixers
        registry.register(Arc::new(BlankLineAfterNamespaceFixer));
        registry.register(Arc::new(NoLeadingImportSlashFixer));
        registry.register(Arc::new(SingleLineAfterImportsFixer));
        registry.register(Arc::new(OrderedImportsFixer));
        registry.register(Arc::new(SingleImportPerStatementFixer));
        registry.register(Arc::new(NoUnusedImportsFixer));

        // Register comment fixers
        registry.register(Arc::new(NoTrailingWhitespaceInCommentFixer));
        registry.register(Arc::new(SingleLineCommentStyleFixer));
        registry.register(Arc::new(MultilineWhitespaceBeforeSemicolonsFixer));

        // Register class/visibility fixers
        registry.register(Arc::new(VisibilityRequiredFixer));
        registry.register(Arc::new(NoBlankLinesAfterClassOpeningFixer));
        registry.register(Arc::new(ClassDefinitionFixer));

        // Sort by priority (descending - higher priority runs first)
        registry.fixers.sort_by(|a, b| b.priority().cmp(&a.priority()));

        // Rebuild indices after sorting
        registry.by_name.clear();
        registry.by_php_name.clear();
        for (idx, fixer) in registry.fixers.iter().enumerate() {
            registry.by_name.insert(fixer.name(), idx);
            registry.by_php_name.insert(fixer.php_cs_fixer_name(), idx);
        }

        registry
    }

    /// Register a fixer
    fn register(&mut self, fixer: Arc<dyn Fixer>) {
        let idx = self.fixers.len();
        self.by_name.insert(fixer.name(), idx);
        self.by_php_name.insert(fixer.php_cs_fixer_name(), idx);
        self.fixers.push(fixer);
    }

    /// Get a fixer by its internal name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Fixer>> {
        self.by_name.get(name).map(|&idx| &self.fixers[idx])
    }

    /// Get a fixer by its PHP-CS-Fixer name
    pub fn get_by_php_name(&self, name: &str) -> Option<&Arc<dyn Fixer>> {
        self.by_php_name.get(name).map(|&idx| &self.fixers[idx])
    }

    /// Get all fixers in priority order
    pub fn all(&self) -> &[Arc<dyn Fixer>] {
        &self.fixers
    }

    /// Get information about all fixers
    pub fn list(&self) -> Vec<FixerInfo> {
        self.fixers
            .iter()
            .map(|f| FixerInfo {
                name: f.name(),
                php_cs_fixer_name: f.php_cs_fixer_name(),
                description: f.description(),
                priority: f.priority(),
                is_risky: f.is_risky(),
            })
            .collect()
    }

    /// Get fixer names that match a preset
    pub fn get_preset_fixers(&self, preset: &str) -> Vec<&'static str> {
        let preset_rules = crate::config::get_preset_rules(preset);
        preset_rules
            .iter()
            .filter(|&&name| self.by_php_name.contains_key(name))
            .copied()
            .collect()
    }

    /// Check source with specified fixers
    pub fn check(
        &self,
        source: &str,
        fixer_names: &[&str],
        config: &FixerConfig,
    ) -> Vec<Edit> {
        let mut all_edits = Vec::new();

        // Get fixers in priority order
        let mut fixers_to_run: Vec<_> = fixer_names
            .iter()
            .filter_map(|name| {
                self.get_by_php_name(name)
                    .or_else(|| self.get(name))
            })
            .collect();

        // Sort by priority (should already be sorted, but ensure correct order)
        fixers_to_run.sort_by(|a, b| b.priority().cmp(&a.priority()));

        for fixer in fixers_to_run {
            let edits = fixer.check(source, config);
            all_edits.extend(edits);
        }

        // Sort edits by position (reverse order for correct application)
        all_edits.sort_by(|a, b| b.start_offset().cmp(&a.start_offset()));

        // Remove overlapping edits (keep higher priority - earlier in list)
        let mut non_overlapping = Vec::new();
        for edit in all_edits {
            let overlaps = non_overlapping.iter().any(|e: &Edit| {
                // Check if ranges overlap
                edit.start_offset() < e.end_offset() && edit.end_offset() > e.start_offset()
            });
            if !overlaps {
                non_overlapping.push(edit);
            }
        }

        non_overlapping
    }

    /// Check source with all fixers
    pub fn check_all(&self, source: &str, config: &FixerConfig) -> Vec<Edit> {
        let names: Vec<&str> = self.fixers.iter().map(|f| f.php_cs_fixer_name()).collect();
        self.check(source, &names, config)
    }

    /// Check source with a preset
    pub fn check_preset(&self, source: &str, preset: &str, config: &FixerConfig) -> Vec<Edit> {
        let names = self.get_preset_fixers(preset);
        self.check(source, &names, config)
    }

    /// Number of registered fixers
    pub fn len(&self) -> usize {
        self.fixers.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.fixers.is_empty()
    }
}

impl Default for FixerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_fixers() {
        let registry = FixerRegistry::new();
        assert!(!registry.is_empty());
        assert!(registry.len() >= 8);
    }

    #[test]
    fn test_get_by_name() {
        let registry = FixerRegistry::new();

        // By internal name
        assert!(registry.get("trailing_whitespace").is_some());
        assert!(registry.get("line_ending").is_some());

        // By PHP-CS-Fixer name
        assert!(registry.get_by_php_name("no_trailing_whitespace").is_some());
        assert!(registry.get_by_php_name("line_ending").is_some());
    }

    #[test]
    fn test_priority_order() {
        let registry = FixerRegistry::new();
        let fixers = registry.all();

        // Verify descending priority order
        for window in fixers.windows(2) {
            assert!(
                window[0].priority() >= window[1].priority(),
                "{} (priority {}) should come before {} (priority {})",
                window[0].name(),
                window[0].priority(),
                window[1].name(),
                window[1].priority()
            );
        }
    }

    #[test]
    fn test_check_simple() {
        let registry = FixerRegistry::new();
        let config = FixerConfig::default();

        // Source with trailing whitespace
        let source = "<?php\n$a = 1;   \n";
        let edits = registry.check(source, &["no_trailing_whitespace"], &config);

        assert!(!edits.is_empty());
    }

    #[test]
    fn test_list_fixers() {
        let registry = FixerRegistry::new();
        let list = registry.list();

        assert!(!list.is_empty());

        // Check that all fixers have required info
        for info in &list {
            assert!(!info.name.is_empty());
            assert!(!info.php_cs_fixer_name.is_empty());
            assert!(!info.description.is_empty());
        }
    }
}
