//! Rule: Convert strpos() !== false to str_contains() (PHP 8.0+)
//!
//! Example:
//! ```php
//! // Before
//! if (strpos($text, 'needle') !== false) { }
//! if (strpos($text, 'needle') === false) { }
//! if (false !== strpos($text, 'needle')) { }
//! if (false === strpos($text, 'needle')) { }
//!
//! // After
//! if (str_contains($text, 'needle')) { }
//! if (!str_contains($text, 'needle')) { }
//! if (str_contains($text, 'needle')) { }
//! if (!str_contains($text, 'needle')) { }
//! ```
//!
//! ## Configuration
//!
//! - `strict_comparison` (bool, default: true): When true, only converts strict
//!   comparisons (=== and !==). When false, also converts loose comparisons (== and !=).

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use std::collections::HashMap;

use crate::registry::{Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule};

/// Configuration for the string_contains rule
#[derive(Debug, Clone)]
pub struct StringContainsConfig {
    /// Only convert strict comparisons (=== and !==) when true
    /// Also convert loose comparisons (== and !=) when false
    pub strict_comparison: bool,
}

impl Default for StringContainsConfig {
    fn default() -> Self {
        Self {
            strict_comparison: true,
        }
    }
}

/// Check a parsed PHP program for strpos() !== false patterns
pub fn check_string_contains<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_string_contains_with_config(program, source, &StringContainsConfig::default())
}

/// Check a parsed PHP program for strpos() patterns with configuration
pub fn check_string_contains_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &StringContainsConfig,
) -> Vec<Edit> {
    let mut visitor = StringContainsVisitor {
        source,
        config,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct StringContainsVisitor<'s, 'c> {
    source: &'s str,
    config: &'c StringContainsConfig,
    edits: Vec<Edit>,
}

impl<'a, 's, 'c> Visitor<'a> for StringContainsVisitor<'s, 'c> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Binary(binary) = expr {
            self.check_binary_expression(binary);
        }
        true // Continue traversal
    }
}

impl<'s, 'c> StringContainsVisitor<'s, 'c> {
    fn check_binary_expression(&mut self, binary: &Binary<'_>) {
        // Match patterns:
        // - strpos($x, $y) !== false (strict)
        // - strpos($x, $y) === false (strict)
        // - strpos($x, $y) != false (loose, when strict_comparison=false)
        // - strpos($x, $y) == false (loose, when strict_comparison=false)
        // - false !== strpos($x, $y) (strict)
        // - false === strpos($x, $y) (strict)

        let is_negated = match &binary.operator {
            BinaryOperator::NotIdentical(_) => true,  // !== false means "contains"
            BinaryOperator::Identical(_) => false,    // === false means "not contains"
            BinaryOperator::NotEqual(_) if !self.config.strict_comparison => true,  // != false
            BinaryOperator::Equal(_) if !self.config.strict_comparison => false,    // == false
            _ => return,
        };

        // Try both orderings
        let (lhs, rhs) = (binary.lhs, binary.rhs);

        // Check strpos($x, $y) !== false
        if let (Some((haystack, needle)), true) = (self.extract_strpos_call(lhs), self.is_false_literal(rhs)) {
            self.create_edit(binary, &haystack, &needle, is_negated);
            return;
        }

        // Check false !== strpos($x, $y)
        if let (true, Some((haystack, needle))) = (self.is_false_literal(lhs), self.extract_strpos_call(rhs)) {
            self.create_edit(binary, &haystack, &needle, is_negated);
        }
    }

    /// Extract haystack and needle from a strpos() call
    fn extract_strpos_call(&self, expr: &Expression<'_>) -> Option<(String, String)> {
        if let Expression::Call(Call::Function(func)) = expr {
            // Check if it's a strpos call
            let name: &str = match func.function {
                Expression::Identifier(ident) => {
                    let span = ident.span();
                    &self.source[span.start.offset as usize..span.end.offset as usize]
                }
                _ => return None,
            };

            if !name.eq_ignore_ascii_case("strpos") {
                return None;
            }

            // Get exactly 2 arguments (no offset parameter)
            let args: Vec<_> = func.argument_list.arguments.iter().collect();
            if args.len() != 2 {
                return None;
            }

            // Skip unpacked arguments
            if args[0].is_unpacked() || args[1].is_unpacked() {
                return None;
            }

            // Extract argument code from source
            let haystack_span = args[0].span();
            let needle_span = args[1].span();

            let haystack = self.source[haystack_span.start.offset as usize..haystack_span.end.offset as usize].to_string();
            let needle = self.source[needle_span.start.offset as usize..needle_span.end.offset as usize].to_string();

            return Some((haystack, needle));
        }
        None
    }

    /// Check if expression is the false literal
    fn is_false_literal(&self, expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::False(_)))
    }

    fn create_edit(
        &mut self,
        binary: &Binary<'_>,
        haystack: &str,
        needle: &str,
        is_negated: bool,
    ) {
        let span = binary.span();

        let replacement = if is_negated {
            // !== false means "contains", so no negation
            format!("str_contains({}, {})", haystack, needle)
        } else {
            // === false means "does not contain", so negate
            format!("!str_contains({}, {})", haystack, needle)
        };

        self.edits.push(Edit::new(
            span,
            replacement,
            "Convert strpos() to str_contains() (PHP 8.0+)",
        ));
    }
}

/// Rule to convert strpos() !== false to str_contains()
pub struct StringContainsRule {
    config: StringContainsConfig,
}

impl StringContainsRule {
    /// Create a new rule with default configuration
    pub fn new() -> Self {
        Self {
            config: StringContainsConfig::default(),
        }
    }
}

impl Default for StringContainsRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for StringContainsRule {
    fn name(&self) -> &'static str {
        "string_contains"
    }

    fn description(&self) -> &'static str {
        "Convert strpos() !== false to str_contains()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_string_contains_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[ConfigOption {
            name: "strict_comparison",
            description: "Only convert strict comparisons (=== and !==). When false, also converts loose comparisons (== and !=).",
            default: "true",
            option_type: ConfigOptionType::Bool,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for StringContainsRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let strict_comparison = config
            .get("strict_comparison")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Self {
            config: StringContainsConfig { strict_comparison },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_string_contains(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Transformation Tests ====================

    #[test]
    fn test_strpos_not_identical_false() {
        let source = r#"<?php
if (strpos($text, 'needle') !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains($text, 'needle')"));
        assert!(!result.contains("!str_contains"));
    }

    #[test]
    fn test_strpos_identical_false() {
        let source = r#"<?php
if (strpos($text, 'needle') === false) {
    echo 'not found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("!str_contains($text, 'needle')"));
    }

    #[test]
    fn test_false_not_identical_strpos() {
        let source = r#"<?php
if (false !== strpos($text, 'needle')) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains($text, 'needle')"));
        assert!(!result.contains("!str_contains"));
    }

    #[test]
    fn test_false_identical_strpos() {
        let source = r#"<?php
if (false === strpos($text, 'needle')) {
    echo 'not found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("!str_contains($text, 'needle')"));
    }

    #[test]
    fn test_variable_needle() {
        let source = r#"<?php
if (strpos($haystack, $needle) !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains($haystack, $needle)"));
    }

    #[test]
    fn test_function_call_as_haystack() {
        let source = r#"<?php
if (strpos(strtolower($text), 'needle') !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains(strtolower($text), 'needle')"));
    }

    #[test]
    fn test_in_ternary() {
        let source = r#"<?php
$result = strpos($text, 'x') !== false ? 'yes' : 'no';
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("str_contains($text, 'x')"));
    }

    #[test]
    fn test_in_return() {
        let source = r#"<?php
return strpos($text, 'x') !== false;
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        let result = transform(source);
        assert!(result.contains("return str_contains($text, 'x')"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_strpos_with_offset() {
        // strpos with offset (3rd argument) shouldn't be converted
        let source = r#"<?php
if (strpos($text, 'needle', 5) !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_strpos_compared_to_number() {
        // strpos compared to a number (checking position) shouldn't be converted
        let source = r#"<?php
if (strpos($text, 'needle') === 0) {
    echo 'starts with';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_strpos_greater_than() {
        let source = r#"<?php
if (strpos($text, 'needle') > 0) {
    echo 'found after start';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_strpos_loose_comparison() {
        // Loose comparison (== or !=) shouldn't be converted
        let source = r#"<?php
if (strpos($text, 'needle') != false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    #[test]
    fn test_skip_stripos() {
        // stripos is case-insensitive, str_contains is case-sensitive
        let source = r#"<?php
if (stripos($text, 'needle') !== false) {
    echo 'found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }

    // ==================== Multiple Occurrences ====================

    #[test]
    fn test_multiple_strpos_checks() {
        let source = r#"<?php
if (strpos($a, 'x') !== false && strpos($b, 'y') !== false) {
    echo 'both found';
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);
        let result = transform(source);
        assert!(result.contains("str_contains($a, 'x')"));
        assert!(result.contains("str_contains($b, 'y')"));
    }

    // ==================== Configuration Tests ====================

    fn check_php_with_config(source: &str, config: &StringContainsConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_string_contains_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &StringContainsConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_loose_comparison_with_config() {
        // By default, loose comparisons are skipped
        let source = r#"<?php
if (strpos($text, 'needle') != false) {
    echo 'found';
}
"#;

        // Default config (strict_comparison=true) should skip
        let edits = check_php_with_config(source, &StringContainsConfig::default());
        assert_eq!(edits.len(), 0);

        // With strict_comparison=false, should convert
        let config = StringContainsConfig {
            strict_comparison: false,
        };
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("str_contains($text, 'needle')"));
    }

    #[test]
    fn test_loose_equal_comparison_with_config() {
        let source = r#"<?php
if (strpos($text, 'needle') == false) {
    echo 'not found';
}
"#;

        // Default config should skip
        let edits = check_php_with_config(source, &StringContainsConfig::default());
        assert_eq!(edits.len(), 0);

        // With strict_comparison=false, should convert
        let config = StringContainsConfig {
            strict_comparison: false,
        };
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("!str_contains($text, 'needle')"));
    }

    #[test]
    fn test_configurable_rule_with_config() {
        let mut config = HashMap::new();
        config.insert("strict_comparison".to_string(), ConfigValue::Bool(false));

        let rule = StringContainsRule::with_config(&config);
        assert!(!rule.config.strict_comparison);

        // Empty config should use defaults
        let rule_default = StringContainsRule::with_config(&HashMap::new());
        assert!(rule_default.config.strict_comparison);
    }

    #[test]
    fn test_config_options_metadata() {
        let rule = StringContainsRule::new();
        let options = rule.config_options();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].name, "strict_comparison");
        assert_eq!(options[0].option_type, ConfigOptionType::Bool);
        assert_eq!(options[0].default, "true");
    }
}
