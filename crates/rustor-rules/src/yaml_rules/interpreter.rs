//! YAML rule interpreter
//!
//! Executes YAML-defined rules against PHP AST without compilation.
//! Implements the Rule trait for integration with the existing registry.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use super::matcher::PatternMatcher;
use super::replacer::Replacer;
use super::schema::YamlRule;
use crate::registry::{Category, PhpVersion, Rule};

/// Interpreter for a single YAML rule
pub struct YamlRuleInterpreter {
    rule: YamlRule,
}

impl YamlRuleInterpreter {
    /// Create a new interpreter for a YAML rule
    pub fn new(rule: YamlRule) -> Self {
        Self { rule }
    }

    /// Get a reference to the underlying rule
    pub fn rule(&self) -> &YamlRule {
        &self.rule
    }

    /// Run tests defined in the YAML rule
    #[cfg(test)]
    pub fn run_tests(&self) -> Vec<TestResult> {
        self.rule
            .tests
            .iter()
            .filter(|t| !t.skip)
            .map(|test| self.run_single_test(test))
            .collect()
    }

    #[cfg(test)]
    fn run_single_test(&self, test: &super::schema::TestCase) -> TestResult {
        use bumpalo::Bump;
        use mago_database::file::FileId;
        use mago_syntax::parser::parse_file_content;

        // Parse the input code
        let full_input = format!("<?php {};", test.input);
        let bump = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = parse_file_content(&bump, file_id, &full_input);

        // Run the rule
        let edits = self.check_internal(program, &full_input);

        if let Some(expected_output) = &test.output {
            // Apply edits and compare
            if edits.is_empty() {
                TestResult {
                    input: test.input.clone(),
                    expected: Some(expected_output.clone()),
                    actual: None,
                    passed: false,
                    error: Some("No edits produced".to_string()),
                }
            } else {
                // Apply the first edit
                let edit = &edits[0];
                let actual = self.apply_edit(&full_input, edit);
                // Extract just the expression part (after "<?php " and before ";")
                let actual_expr = actual
                    .strip_prefix("<?php ")
                    .and_then(|s| s.strip_suffix(';'))
                    .unwrap_or(&actual);

                let passed = actual_expr == expected_output;
                TestResult {
                    input: test.input.clone(),
                    expected: Some(expected_output.clone()),
                    actual: Some(actual_expr.to_string()),
                    passed,
                    error: None,
                }
            }
        } else {
            // No expected output means the rule should not match
            let passed = edits.is_empty();
            TestResult {
                input: test.input.clone(),
                expected: None,
                actual: if edits.is_empty() {
                    None
                } else {
                    Some(edits[0].replacement.clone())
                },
                passed,
                error: if passed {
                    None
                } else {
                    Some("Expected no match but rule matched".to_string())
                },
            }
        }
    }

    #[cfg(test)]
    fn apply_edit(&self, source: &str, edit: &Edit) -> String {
        let start = edit.span.start.offset as usize;
        let end = edit.span.end.offset as usize;
        format!("{}{}{}", &source[..start], edit.replacement, &source[end..])
    }

    fn check_internal<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        let mut visitor = YamlRuleVisitor {
            source,
            rule: &self.rule,
            edits: Vec::new(),
        };
        visitor.visit_program(program, source);
        visitor.edits
    }
}

/// Result of running a single test case
#[cfg(test)]
#[derive(Debug)]
pub struct TestResult {
    pub input: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub passed: bool,
    pub error: Option<String>,
}

impl Rule for YamlRuleInterpreter {
    fn name(&self) -> &'static str {
        // Leak the string to get a 'static reference
        // This is acceptable because rules are typically long-lived
        Box::leak(self.rule.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(self.rule.description.clone().into_boxed_str())
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        self.check_internal(program, source)
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        self.rule.min_php.as_ref().and_then(|v| {
            match v.as_str() {
                "5.4" => Some(PhpVersion::Php54),
                "5.5" => Some(PhpVersion::Php55),
                "5.6" => Some(PhpVersion::Php56),
                "7.0" => Some(PhpVersion::Php70),
                "7.1" => Some(PhpVersion::Php71),
                "7.2" => Some(PhpVersion::Php72),
                "7.3" => Some(PhpVersion::Php73),
                "7.4" => Some(PhpVersion::Php74),
                "8.0" => Some(PhpVersion::Php80),
                "8.1" => Some(PhpVersion::Php81),
                "8.2" => Some(PhpVersion::Php82),
                "8.3" => Some(PhpVersion::Php83),
                "8.4" => Some(PhpVersion::Php84),
                _ => None,
            }
        })
    }

    fn category(&self) -> Category {
        match self.rule.category.to_lowercase().as_str() {
            "performance" => Category::Performance,
            "modernization" | "modernize" | "php70" | "php71" | "php72" | "php73" | "php74"
            | "php80" | "php81" | "php82" | "php83" | "php84" => Category::Modernization,
            "compatibility" => Category::Compatibility,
            _ => Category::Simplification,
        }
    }
}

/// Visitor that applies a YAML rule to the AST
struct YamlRuleVisitor<'s, 'r> {
    source: &'s str,
    rule: &'r YamlRule,
    edits: Vec<Edit>,
}

impl<'a, 's, 'r> Visitor<'a> for YamlRuleVisitor<'s, 'r> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        let matcher = PatternMatcher::new(self.source);

        // Try to match the pattern
        if let Some(bindings) = matcher.match_expression(&self.rule.match_pattern, expr) {
            // Check conditions (when clauses)
            if self.check_conditions(&bindings) {
                // Apply replacement
                if let Some(replacement_text) = Replacer::apply(&self.rule.replace, &bindings) {
                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement_text,
                        self.rule.description.clone(),
                    ));
                    // Don't visit children if we matched
                    return false;
                }
            }
        }

        // Continue visiting children
        true
    }

    fn visit_statement(&mut self, _stmt: &Statement<'a>, _source: &str) -> bool {
        // Let the default visitor implementation handle statement traversal
        // We only need to override visit_expression for pattern matching
        true
    }
}

impl<'s, 'r> YamlRuleVisitor<'s, 'r> {
    fn check_conditions(&self, bindings: &super::matcher::CapturedBindings) -> bool {
        // If no conditions, rule always applies
        if self.rule.when.is_empty() {
            return true;
        }

        // All conditions must be satisfied
        for condition in &self.rule.when {
            match condition {
                super::schema::RuleCondition::Type { var, type_is: _ } => {
                    // Type checking requires PHPStan integration
                    // For now, just check if the variable was captured
                    let var_name = var.strip_prefix('$').unwrap_or(var);
                    if !bindings.contains(var_name) {
                        return false;
                    }
                }
                super::schema::RuleCondition::Value { var, value } => {
                    let var_name = var.strip_prefix('$').unwrap_or(var);
                    if let Some(captured) = bindings.get_text(var_name) {
                        // Simple value comparison
                        if !Self::check_value_condition(captured, value) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                super::schema::RuleCondition::Context(_ctx) => {
                    // Context conditions (in_class, in_function, etc.)
                    // Would require additional context tracking
                    // For now, pass through
                }
            }
        }

        true
    }

    fn check_value_condition(captured: &str, condition: &str) -> bool {
        let condition = condition.trim();

        // Handle numeric comparisons
        if condition.starts_with('>') {
            let num = condition[1..].trim().parse::<i64>().unwrap_or(0);
            return captured.parse::<i64>().unwrap_or(0) > num;
        }
        if condition.starts_with('<') {
            let num = condition[1..].trim().parse::<i64>().unwrap_or(0);
            return captured.parse::<i64>().unwrap_or(0) < num;
        }
        if condition.starts_with(">=") {
            let num = condition[2..].trim().parse::<i64>().unwrap_or(0);
            return captured.parse::<i64>().unwrap_or(0) >= num;
        }
        if condition.starts_with("<=") {
            let num = condition[2..].trim().parse::<i64>().unwrap_or(0);
            return captured.parse::<i64>().unwrap_or(0) <= num;
        }
        if condition.starts_with("==") {
            let expected = condition[2..].trim();
            return captured == expected;
        }
        if condition.starts_with("!=") {
            let expected = condition[2..].trim();
            return captured != expected;
        }

        // Handle regex match
        if condition.starts_with("matches(") && condition.ends_with(')') {
            let pattern = &condition[8..condition.len() - 1];
            let pattern = pattern.trim_matches('/');
            if let Ok(re) = regex::Regex::new(pattern) {
                return re.is_match(captured);
            }
        }

        // Default: exact match
        captured == condition
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rule() -> YamlRule {
        serde_yaml::from_str(
            r#"
name: test_is_null
description: Replace is_null($x) with $x === null
category: code_quality
min_php: "7.0"

match:
  node: FuncCall
  name: is_null
  args:
    - capture: $expr

replace: "$expr === null"

tests:
  - input: "is_null($x)"
    output: "$x === null"
  - input: "is_null($obj->prop)"
    output: "$obj->prop === null"
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_interpreter_rule_metadata() {
        let rule = create_test_rule();
        let interpreter = YamlRuleInterpreter::new(rule);

        assert_eq!(interpreter.name(), "test_is_null");
        assert_eq!(interpreter.min_php_version(), Some(PhpVersion::Php70));
    }

    #[test]
    fn test_interpreter_runs_tests() {
        let rule = create_test_rule();
        let interpreter = YamlRuleInterpreter::new(rule);

        let results = interpreter.run_tests();
        assert_eq!(results.len(), 2);

        for result in &results {
            assert!(result.passed, "Test failed: {:?}", result);
        }
    }

    #[test]
    fn test_interpreter_check() {
        use bumpalo::Bump;
        use mago_database::file::FileId;
        use mago_syntax::parser::parse_file_content;

        let rule = create_test_rule();
        let interpreter = YamlRuleInterpreter::new(rule);

        let code = "<?php is_null($x);";
        let bump = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = parse_file_content(&bump, file_id, code);

        let edits = interpreter.check(program, code);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "$x === null");
    }
}
