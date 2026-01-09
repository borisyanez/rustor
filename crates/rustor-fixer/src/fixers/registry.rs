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
    AlignMultilineCommentFixer,
    NoExtraBlankLinesFixer,
    NoLeadingNamespaceWhitespaceFixer,
    NoMultilineWhitespaceAroundDoubleArrowFixer,
    NoSinglelineWhitespaceBeforeSemicolonsFixer,
    NoSpacesAroundOffsetFixer,
    NoWhitespaceBeforeCommaInArrayFixer,
    SpaceAfterSemicolonFixer,
    TrimArraySpacesFixer,
    TypeDeclarationSpacesFixer,
    WhitespaceAfterCommaInArrayFixer,
    StatementIndentationFixer,
    ArrayIndentationFixer,
    LinebreakAfterOpeningTagFixer,
};
use super::casing::{
    LowercaseKeywordsFixer,
    ConstantCaseFixer,
    LowercaseStaticReferenceFixer,
    NativeFunctionCasingFixer,
    MagicMethodCasingFixer,
    MagicConstantCasingFixer,
    LowercaseCastFixer,
};
use super::braces::{
    ElseifFixer,
    NoClosingTagFixer,
    SwitchCaseSpaceFixer,
    BracesPositionFixer,
    SwitchCaseSemicolonToColonFixer,
    DeclareEqualNormalizeFixer,
    ControlStructureBracesFixer,
    ControlStructureContinuationPositionFixer,
    NoAlternativeSyntaxFixer,
    NoUnneededBracesFixer,
    SingleLineEmptyBodyFixer,
};
use super::functions::{
    FunctionDeclarationFixer,
    MethodArgumentSpaceFixer,
    ReturnTypeDeclarationFixer,
    CompactNullableTypeDeclarationFixer,
    NoSpacesAfterFunctionNameFixer,
};
use super::operators::{
    ConcatSpaceFixer,
    BinaryOperatorSpacesFixer,
    UnaryOperatorSpacesFixer,
    MethodChainingIndentationFixer,
    NewWithParenthesesFixer,
    NoSpaceAroundDoubleColonFixer,
    ObjectOperatorWithoutWhitespaceFixer,
    TernaryOperatorSpacesFixer,
    StandardizeIncrementFixer,
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
    SingleLineCommentSpacingFixer,
};
use super::class::{
    VisibilityRequiredFixer,
    NoBlankLinesAfterClassOpeningFixer,
    ClassDefinitionFixer,
    SingleClassElementPerStatementFixer,
    SingleTraitInsertPerStatementFixer,
    OrderedClassElementsFixer,
};
use super::risky::{
    StrictComparisonFixer,
    DeclareStrictTypesFixer,
    NoAliasFunctionsFixer,
};
use super::syntax::{
    ArraySyntaxFixer,
    SingleQuoteFixer,
    YodaStyleFixer,
    EchoTagSyntaxFixer,
    BacktickToShellExecFixer,
    NormalizeIndexBraceFixer,
    IncrementStyleFixer,
    StandardizeNotEqualsFixer,
    ClassAttributesSeparationFixer,
    ClassReferenceNameCasingFixer,
    CleanNamespaceFixer,
    DeclareParenthesesFixer,
    EmptyLoopBodyFixer,
    EmptyLoopConditionFixer,
    IncludeFixer,
    IntegerLiteralCaseFixer,
    NoAliasLanguageConstructCallFixer,
    NoBinaryStringFixer,
    OperatorLinebreakFixer,
    SingleSpaceAroundConstructFixer,
};
use super::cleanup::{
    NoEmptyStatementFixer,
    NoUselessElseFixer,
    NoUselessReturnFixer,
    NoEmptyCommentFixer,
    NoEmptyPhpdocFixer,
    NoShortBoolCastFixer,
    NoUnsetCastFixer,
    NoUselessConcatOperatorFixer,
    NoUselessNullsafeOperatorFixer,
    SimplifiedIfReturnFixer,
    SimplifiedNullReturnFixer,
    NoSuperfluousElseifFixer,
    NoUnneededControlParenthesesFixer,
    NoUnneededImportAliasFixer,
    SwitchContinueToBreakFixer,
    GlobalNamespaceImportFixer,
    LambdaNotUsedImportFixer,
    NativeTypeDeclarationCasingFixer,
    NoTrailingCommaInSinglelineFixer,
    SingleLineThrowFixer,
};
use super::phpdoc::{
    PhpdocAlignFixer,
    PhpdocIndentFixer,
    PhpdocScalarTypeFixer,
    PhpdocSeparationFixer,
    PhpdocSingleLineVarSpacingFixer,
    PhpdocSummaryFixer,
    PhpdocTrimFixer,
    PhpdocTrimConsecutiveBlankLineSeparationFixer,
    PhpdocTypesOrderFixer,
    PhpdocVarWithoutNameFixer,
    PhpdocNoEmptyReturnFixer,
    PhpdocOrderFixer,
    PhpdocOrderByValueFixer,
    PhpdocReturnSelfReferenceFixer,
    PhpdocNoAliasTagFixer,
    PhpdocNoPackageFixer,
    PhpdocNoUselessInheritdocFixer,
    PhpdocTagTypeFixer,
    PhpdocLineSpanFixer,
    PhpdocArrayTypeFixer,
    GeneralPhpdocAnnotationRemoveFixer,
    GeneralPhpdocTagRenameFixer,
    NoBlankLinesAfterPhpdocFixer,
    PhpdocAnnotationWithoutDotFixer,
    PhpdocInlineTagNormalizerFixer,
    PhpdocNoAccessFixer,
    PhpdocToCommentFixer,
    PhpdocTypesFixer,
    PhpdocVarAnnotationCorrectOrderFixer,
};
use super::types::{
    NullableTypeDeclarationFixer,
    VoidReturnFixer,
    OrderedTypesFixer,
    NoSuperfluousPhpdocTagsFixer,
    FullyQualifiedStrictTypesFixer,
    NullableTypeDeclarationForDefaultNullValueFixer,
    UnionTypeDeclarationFixer,
    NoNullPropertyInitializationFixer,
};
use super::misc::{
    CastSpacesFixer,
    TrailingCommaInMultilineFixer,
    BlankLineBeforeStatementFixer,
    CombineConsecutiveIssetsFixer,
    CombineConsecutiveUnsetsFixer,
    ExplicitStringVariableFixer,
    HeredocToNowdocFixer,
    ListSyntaxFixer,
    MultilineCommentOpeningClosingFixer,
    NoMultipleStatementsPerLineFixer,
    SemicolonAfterInstructionFixer,
    TernaryToNullCoalescingFixer,
    AssignNullCoalescingToCoalesceEqualFixer,
    SimpleToComplexStringVariableFixer,
    PhpUnitFqcnAnnotationFixer,
    PhpUnitMethodCasingFixer,
    PhpUnitTestAnnotationFixer,
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
        registry.register(Arc::new(AlignMultilineCommentFixer));
        registry.register(Arc::new(NoExtraBlankLinesFixer));
        registry.register(Arc::new(NoLeadingNamespaceWhitespaceFixer));
        registry.register(Arc::new(NoMultilineWhitespaceAroundDoubleArrowFixer));
        registry.register(Arc::new(NoSinglelineWhitespaceBeforeSemicolonsFixer));
        registry.register(Arc::new(NoSpacesAroundOffsetFixer));
        registry.register(Arc::new(NoWhitespaceBeforeCommaInArrayFixer));
        registry.register(Arc::new(SpaceAfterSemicolonFixer));
        registry.register(Arc::new(TrimArraySpacesFixer));
        registry.register(Arc::new(TypeDeclarationSpacesFixer));
        registry.register(Arc::new(WhitespaceAfterCommaInArrayFixer));
        registry.register(Arc::new(StatementIndentationFixer));
        registry.register(Arc::new(ArrayIndentationFixer));
        registry.register(Arc::new(LinebreakAfterOpeningTagFixer));

        // Register casing fixers
        registry.register(Arc::new(LowercaseKeywordsFixer));
        registry.register(Arc::new(ConstantCaseFixer));
        registry.register(Arc::new(LowercaseStaticReferenceFixer));
        registry.register(Arc::new(NativeFunctionCasingFixer));
        registry.register(Arc::new(MagicMethodCasingFixer));
        registry.register(Arc::new(MagicConstantCasingFixer));
        registry.register(Arc::new(LowercaseCastFixer));

        // Register braces/control structure fixers
        registry.register(Arc::new(NoClosingTagFixer));
        registry.register(Arc::new(ElseifFixer));
        registry.register(Arc::new(SwitchCaseSpaceFixer));
        registry.register(Arc::new(BracesPositionFixer));
        registry.register(Arc::new(SwitchCaseSemicolonToColonFixer));
        registry.register(Arc::new(DeclareEqualNormalizeFixer));
        registry.register(Arc::new(ControlStructureBracesFixer));
        registry.register(Arc::new(ControlStructureContinuationPositionFixer));
        registry.register(Arc::new(NoAlternativeSyntaxFixer));
        registry.register(Arc::new(NoUnneededBracesFixer));
        registry.register(Arc::new(SingleLineEmptyBodyFixer));

        // Register function fixers
        registry.register(Arc::new(FunctionDeclarationFixer));
        registry.register(Arc::new(MethodArgumentSpaceFixer));
        registry.register(Arc::new(ReturnTypeDeclarationFixer));
        registry.register(Arc::new(CompactNullableTypeDeclarationFixer));
        registry.register(Arc::new(NoSpacesAfterFunctionNameFixer));

        // Register operator fixers
        registry.register(Arc::new(ConcatSpaceFixer));
        registry.register(Arc::new(BinaryOperatorSpacesFixer));
        registry.register(Arc::new(UnaryOperatorSpacesFixer));
        registry.register(Arc::new(MethodChainingIndentationFixer));
        registry.register(Arc::new(NewWithParenthesesFixer));
        registry.register(Arc::new(NoSpaceAroundDoubleColonFixer));
        registry.register(Arc::new(ObjectOperatorWithoutWhitespaceFixer));
        registry.register(Arc::new(TernaryOperatorSpacesFixer));
        registry.register(Arc::new(StandardizeIncrementFixer));

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
        registry.register(Arc::new(SingleLineCommentSpacingFixer));

        // Register class/visibility fixers
        registry.register(Arc::new(VisibilityRequiredFixer));
        registry.register(Arc::new(NoBlankLinesAfterClassOpeningFixer));
        registry.register(Arc::new(ClassDefinitionFixer));
        registry.register(Arc::new(SingleClassElementPerStatementFixer));
        registry.register(Arc::new(SingleTraitInsertPerStatementFixer));
        registry.register(Arc::new(OrderedClassElementsFixer));

        // Register risky fixers (not enabled by default)
        registry.register(Arc::new(StrictComparisonFixer));
        registry.register(Arc::new(DeclareStrictTypesFixer));
        registry.register(Arc::new(NoAliasFunctionsFixer));

        // Register syntax fixers
        registry.register(Arc::new(ArraySyntaxFixer));
        registry.register(Arc::new(SingleQuoteFixer));
        registry.register(Arc::new(YodaStyleFixer));
        registry.register(Arc::new(EchoTagSyntaxFixer));
        registry.register(Arc::new(BacktickToShellExecFixer));
        registry.register(Arc::new(NormalizeIndexBraceFixer));
        registry.register(Arc::new(IncrementStyleFixer));
        registry.register(Arc::new(StandardizeNotEqualsFixer));
        registry.register(Arc::new(ClassAttributesSeparationFixer));
        registry.register(Arc::new(ClassReferenceNameCasingFixer));
        registry.register(Arc::new(CleanNamespaceFixer));
        registry.register(Arc::new(DeclareParenthesesFixer));
        registry.register(Arc::new(EmptyLoopBodyFixer));
        registry.register(Arc::new(EmptyLoopConditionFixer));
        registry.register(Arc::new(IncludeFixer));
        registry.register(Arc::new(IntegerLiteralCaseFixer));
        registry.register(Arc::new(NoAliasLanguageConstructCallFixer));
        registry.register(Arc::new(NoBinaryStringFixer));
        registry.register(Arc::new(OperatorLinebreakFixer));
        registry.register(Arc::new(SingleSpaceAroundConstructFixer));

        // Register cleanup fixers
        registry.register(Arc::new(NoEmptyStatementFixer));
        registry.register(Arc::new(NoUselessElseFixer));
        registry.register(Arc::new(NoUselessReturnFixer));
        registry.register(Arc::new(NoEmptyCommentFixer));
        registry.register(Arc::new(NoEmptyPhpdocFixer));
        registry.register(Arc::new(NoShortBoolCastFixer));
        registry.register(Arc::new(NoUnsetCastFixer));
        registry.register(Arc::new(NoUselessConcatOperatorFixer));
        registry.register(Arc::new(NoUselessNullsafeOperatorFixer));
        registry.register(Arc::new(SimplifiedIfReturnFixer));
        registry.register(Arc::new(SimplifiedNullReturnFixer));
        registry.register(Arc::new(NoSuperfluousElseifFixer));
        registry.register(Arc::new(NoUnneededControlParenthesesFixer));
        registry.register(Arc::new(NoUnneededImportAliasFixer));
        registry.register(Arc::new(SwitchContinueToBreakFixer));
        registry.register(Arc::new(GlobalNamespaceImportFixer));
        registry.register(Arc::new(LambdaNotUsedImportFixer));
        registry.register(Arc::new(NativeTypeDeclarationCasingFixer));
        registry.register(Arc::new(NoTrailingCommaInSinglelineFixer));
        registry.register(Arc::new(SingleLineThrowFixer));

        // Register PHPDoc fixers
        registry.register(Arc::new(PhpdocAlignFixer));
        registry.register(Arc::new(PhpdocIndentFixer));
        registry.register(Arc::new(PhpdocScalarTypeFixer));
        registry.register(Arc::new(PhpdocSeparationFixer));
        registry.register(Arc::new(PhpdocSingleLineVarSpacingFixer));
        registry.register(Arc::new(PhpdocSummaryFixer));
        registry.register(Arc::new(PhpdocTrimFixer));
        registry.register(Arc::new(PhpdocTrimConsecutiveBlankLineSeparationFixer));
        registry.register(Arc::new(PhpdocTypesOrderFixer));
        registry.register(Arc::new(PhpdocVarWithoutNameFixer));
        registry.register(Arc::new(PhpdocNoEmptyReturnFixer));
        registry.register(Arc::new(PhpdocOrderFixer));
        registry.register(Arc::new(PhpdocOrderByValueFixer));
        registry.register(Arc::new(PhpdocReturnSelfReferenceFixer));
        registry.register(Arc::new(PhpdocNoAliasTagFixer));
        registry.register(Arc::new(PhpdocNoPackageFixer));
        registry.register(Arc::new(PhpdocNoUselessInheritdocFixer));
        registry.register(Arc::new(PhpdocTagTypeFixer));
        registry.register(Arc::new(PhpdocLineSpanFixer));
        registry.register(Arc::new(PhpdocArrayTypeFixer));
        registry.register(Arc::new(GeneralPhpdocAnnotationRemoveFixer));
        registry.register(Arc::new(GeneralPhpdocTagRenameFixer));
        registry.register(Arc::new(NoBlankLinesAfterPhpdocFixer));
        registry.register(Arc::new(PhpdocAnnotationWithoutDotFixer));
        registry.register(Arc::new(PhpdocInlineTagNormalizerFixer));
        registry.register(Arc::new(PhpdocNoAccessFixer));
        registry.register(Arc::new(PhpdocToCommentFixer));
        registry.register(Arc::new(PhpdocTypesFixer));
        registry.register(Arc::new(PhpdocVarAnnotationCorrectOrderFixer));

        // Register type fixers
        registry.register(Arc::new(NullableTypeDeclarationFixer));
        registry.register(Arc::new(VoidReturnFixer));
        registry.register(Arc::new(OrderedTypesFixer));
        registry.register(Arc::new(NoSuperfluousPhpdocTagsFixer));
        registry.register(Arc::new(FullyQualifiedStrictTypesFixer));
        registry.register(Arc::new(NullableTypeDeclarationForDefaultNullValueFixer));
        registry.register(Arc::new(UnionTypeDeclarationFixer));
        registry.register(Arc::new(NoNullPropertyInitializationFixer));

        // Register misc fixers
        registry.register(Arc::new(CastSpacesFixer));
        registry.register(Arc::new(TrailingCommaInMultilineFixer));
        registry.register(Arc::new(BlankLineBeforeStatementFixer));
        registry.register(Arc::new(CombineConsecutiveIssetsFixer));
        registry.register(Arc::new(CombineConsecutiveUnsetsFixer));
        registry.register(Arc::new(ExplicitStringVariableFixer));
        registry.register(Arc::new(HeredocToNowdocFixer));
        registry.register(Arc::new(ListSyntaxFixer));
        registry.register(Arc::new(MultilineCommentOpeningClosingFixer));
        registry.register(Arc::new(NoMultipleStatementsPerLineFixer));
        registry.register(Arc::new(SemicolonAfterInstructionFixer));
        registry.register(Arc::new(TernaryToNullCoalescingFixer));
        registry.register(Arc::new(AssignNullCoalescingToCoalesceEqualFixer));
        registry.register(Arc::new(SimpleToComplexStringVariableFixer));
        registry.register(Arc::new(PhpUnitFqcnAnnotationFixer));
        registry.register(Arc::new(PhpUnitMethodCasingFixer));
        registry.register(Arc::new(PhpUnitTestAnnotationFixer));

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
    ///
    /// Runs fixers sequentially in priority order, applying each fixer's edits
    /// to the source before running the next fixer. This ensures fixers see
    /// the already-modified code, matching PHP-CS-Fixer behavior.
    ///
    /// Returns (fixed_source, edits) where edits are for display purposes only.
    pub fn check(
        &self,
        source: &str,
        fixer_names: &[&str],
        config: &FixerConfig,
    ) -> (String, Vec<Edit>) {
        use rustor_core::apply_edits;

        let mut all_edits = Vec::new();
        let mut current_source = source.to_string();

        // Get fixers in priority order
        let mut fixers_to_run: Vec<_> = fixer_names
            .iter()
            .filter_map(|name| {
                self.get_by_php_name(name)
                    .or_else(|| self.get(name))
            })
            .collect();

        // Sort by priority (higher priority runs first)
        fixers_to_run.sort_by(|a, b| b.priority().cmp(&a.priority()));

        for fixer in fixers_to_run {
            let edits = fixer.check(&current_source, config);

            if !edits.is_empty() {
                // Apply this fixer's edits to the current source
                if let Ok(new_source) = apply_edits(&current_source, &edits) {
                    // Track edits (positions will be relative to source at time of edit)
                    all_edits.extend(edits);
                    current_source = new_source;
                }
            }
        }

        (current_source, all_edits)
    }

    /// Check source with all fixers
    pub fn check_all(&self, source: &str, config: &FixerConfig) -> (String, Vec<Edit>) {
        let names: Vec<&str> = self.fixers.iter().map(|f| f.php_cs_fixer_name()).collect();
        self.check(source, &names, config)
    }

    /// Check source with a preset
    pub fn check_preset(&self, source: &str, preset: &str, config: &FixerConfig) -> (String, Vec<Edit>) {
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
        let (_, edits) = registry.check(source, &["no_trailing_whitespace"], &config);

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

    #[test]
    fn test_for_loop_braces_position() {
        use crate::config::LineEnding;

        let registry = FixerRegistry::new();
        let config = FixerConfig {
            line_ending: LineEnding::Lf,
            ..Default::default()
        };

        let source = "<?php\nfor($i=0;$i<10;$i++)\n{\n}\n";

        // Check combined via preset - should include braces_position
        let (fixed_source, all_edits) = registry.check_preset(source, "psr12", &config);

        // Should have braces_position edit
        assert!(
            all_edits.iter().any(|e| e.rule.as_deref() == Some("braces_position")),
            "Expected braces_position edit in combined result"
        );

        // The fixed source should have the for brace on the same line
        assert!(
            fixed_source.contains("for (") && fixed_source.contains(") {"),
            "Expected for loop to have brace on same line, got: {}", fixed_source
        );
    }

    #[test]
    fn test_single_quote_in_symfony_preset() {
        let registry = FixerRegistry::new();
        let config = FixerConfig::default();

        // single_quote is in Symfony preset, not PSR-12 (PSR-12 doesn't mandate quote style)
        let symfony_fixers = registry.get_preset_fixers("symfony");
        assert!(
            symfony_fixers.contains(&"single_quote"),
            "Expected single_quote in symfony preset, got: {:?}", symfony_fixers
        );

        // Test that single_quote fixer is actually applied via symfony preset
        let source = r#"<?php
$a = "simple string";
"#;
        let (fixed, edits) = registry.check_preset(source, "symfony", &config);

        assert!(
            edits.iter().any(|e| e.rule.as_deref() == Some("single_quote")),
            "Expected single_quote edit, got edits: {:?}", edits.iter().map(|e| e.rule.as_deref()).collect::<Vec<_>>()
        );
        assert!(
            fixed.contains("'simple string'"),
            "Expected single quotes in output, got: {}", fixed
        );
    }

    #[test]
    fn test_single_quote_direct() {
        let registry = FixerRegistry::new();
        let config = FixerConfig::default();

        // Test calling single_quote directly
        let source = r#"<?php
$a = "simple string";
"#;
        let (fixed, edits) = registry.check(source, &["single_quote"], &config);

        assert!(
            !edits.is_empty(),
            "Expected single_quote edits"
        );
        assert!(
            fixed.contains("'simple string'"),
            "Expected single quotes in output, got: {}", fixed
        );
    }
}
