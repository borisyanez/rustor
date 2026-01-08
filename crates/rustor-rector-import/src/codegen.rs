//! Code generation - transforms RectorRule into Rust source code
//!
//! This module uses Handlebars templates to generate rustor Rust rules
//! from extracted Rector rule patterns.

use crate::templates::{self, RULE_TEMPLATE, MOD_TEMPLATE};
use crate::{RectorRule, RulePattern};
use convert_case::{Case, Casing};
use handlebars::Handlebars;
use serde_json::json;
use std::fs;
use std::path::Path;

/// Generated rule output
#[derive(Debug)]
pub struct GeneratedRule {
    /// File name (e.g., "is_null.rs")
    pub filename: String,

    /// Generated Rust source code
    pub source: String,

    /// Whether this was auto-generated or needs manual review
    pub needs_review: bool,

    /// Original Rector rule for reference
    pub original_name: String,
}

/// Code generator for rustor rules
pub struct CodeGenerator {
    handlebars: Handlebars<'static>,
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();

        // Disable HTML escaping since we're generating Rust code, not HTML
        handlebars.register_escape_fn(handlebars::no_escape);

        // Register templates
        handlebars
            .register_template_string("rule", RULE_TEMPLATE)
            .expect("Failed to register rule template");
        handlebars
            .register_template_string("mod", MOD_TEMPLATE)
            .expect("Failed to register mod template");

        Self { handlebars }
    }

    /// Generate Rust code for a single rule
    pub fn generate_rule(&self, rule: &RectorRule) -> Result<GeneratedRule, String> {
        let snake_name = rule_to_snake_name(&rule.name);
        let pascal_name = rule_to_pascal_name(&rule.name);

        // Generate visitor implementation based on pattern
        let (visitor_impl, needs_review) = self.generate_visitor_impl(rule);

        // Map category
        let category = map_category(&rule.category);

        // Map PHP version
        let php_version = rule.min_php_version.as_ref()
            .map(|v| format!("Some(PhpVersion::Php{})", v.replace('.', "")))
            .unwrap_or_else(|| "None".to_string());

        // Build template data
        let data = json!({
            "snake_name": snake_name,
            "pascal_name": pascal_name,
            "description": escape_string(&rule.description),
            "before_code": escape_code_sample(&rule.before_code),
            "after_code": escape_code_sample(&rule.after_code),
            "source_file": rule.source_file,
            "visitor_impl": visitor_impl,
            "category": category,
            "php_version": php_version,
        });

        let source = self.handlebars
            .render("rule", &data)
            .map_err(|e| format!("Template error: {}", e))?;

        Ok(GeneratedRule {
            filename: format!("{}.rs", snake_name),
            source,
            needs_review,
            original_name: rule.name.clone(),
        })
    }

    /// Generate the visitor implementation for a rule
    fn generate_visitor_impl(&self, rule: &RectorRule) -> (String, bool) {
        match &rule.pattern {
            RulePattern::FunctionRename { from, to } => {
                let impl_code = templates::VISITOR_FUNCTION_RENAME
                    .replace("{{from_func}}", from)
                    .replace("{{to_func}}", to)
                    .replace("{{description}}", &rule.description);
                (impl_code, false)
            }

            RulePattern::FunctionAlias { from, to } => {
                let impl_code = templates::VISITOR_FUNCTION_ALIAS
                    .replace("{{from_func}}", from)
                    .replace("{{to_func}}", to);
                (impl_code, false)
            }

            RulePattern::FunctionToComparison { func, operator, compare_value } => {
                let impl_code = templates::VISITOR_FUNCTION_TO_COMPARISON
                    .replace("{{func}}", func)
                    .replace("{{operator}}", operator)
                    .replace("{{compare_value}}", compare_value);
                (impl_code, false)
            }

            RulePattern::FunctionToCast { func, cast_type } => {
                let impl_code = templates::VISITOR_FUNCTION_TO_CAST
                    .replace("{{func}}", func)
                    .replace("{{cast_type}}", cast_type);
                (impl_code, false)
            }

            RulePattern::FunctionToOperator { func, operator, .. } => {
                let impl_code = templates::VISITOR_FUNCTION_TO_OPERATOR
                    .replace("{{func}}", func)
                    .replace("{{operator}}", operator);
                (impl_code, false)
            }

            RulePattern::TernaryToCoalesce { condition_func } => {
                let impl_code = templates::VISITOR_TERNARY_TO_COALESCE
                    .replace("{{condition_func}}", condition_func);
                (impl_code, false)
            }

            RulePattern::ArraySyntaxModern => {
                let impl_code = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Match array() syntax
        if let Expression::Array(array) = expr {
            if let ArrayExpression::LegacyConstruct(legacy) = array {
                // Replace array(...) with [...]
                let inner_start = legacy.left_parenthesis.start.offset;
                let inner_end = legacy.right_parenthesis.end.offset;
                let items = &self.source[inner_start + 1..inner_end - 1];

                self.edits.push(Edit {
                    start: expr.span().start.offset,
                    end: expr.span().end.offset,
                    replacement: format!("[{}]", items),
                    message: "Use short array syntax".to_string(),
                });
            }
        }
        true
    }"#.to_string();
                (impl_code, false)
            }

            RulePattern::ClosureToArrow => {
                let impl_code = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // TODO: Implement closure to arrow function conversion
        // This is a complex transformation that needs to:
        // 1. Check if closure has single return statement
        // 2. Check if closure doesn't use $this (unless inherited)
        // 3. Convert to fn($x) => expression syntax
        //
        // For now, mark as needs review
        let _ = expr;
        true
    }"#.to_string();
                (impl_code, true)
            }

            RulePattern::FunctionToClassConstant { func } => {
                let impl_code = templates::VISITOR_FUNCTION_TO_CLASS_CONSTANT
                    .replace("{{func}}", func);
                (impl_code, false)
            }

            RulePattern::FunctionToInstanceof { func } => {
                let impl_code = templates::VISITOR_FUNCTION_TO_INSTANCEOF
                    .replace("{{func}}", func);
                (impl_code, false)
            }

            RulePattern::UnwrapSingleArgFunction { func } => {
                let impl_code = templates::VISITOR_UNWRAP_SINGLE_ARG
                    .replace("{{func}}", func);
                (impl_code, false)
            }

            RulePattern::FunctionRemoveFirstArg { func } => {
                // This pattern needs more context - generate skeleton
                let impl_code = format!(
                    r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {{
        if let Expression::Call(Call::Function(call)) = expr {{
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{}") {{
                // TODO: Remove first argument pattern
                // Implementation depends on specific function semantics
            }}
        }}
        true
    }}"#,
                    func
                );
                (impl_code, true)
            }

            RulePattern::FunctionNoArgsToFunction { from, to } => {
                let impl_code = templates::VISITOR_FUNCTION_NO_ARGS
                    .replace("{{from_func}}", from)
                    .replace("{{to_func}}", to);
                (impl_code, false)
            }

            RulePattern::NullsafeMethodCall => {
                let impl_code = templates::VISITOR_NULLSAFE_METHOD_CALL.to_string();
                (impl_code, true) // Complex pattern, needs review
            }

            RulePattern::FirstClassCallable => {
                let impl_code = templates::VISITOR_FIRST_CLASS_CALLABLE.to_string();
                (impl_code, true) // Complex pattern, needs review
            }

            RulePattern::Complex { hints, refactor_body } => {
                let hints_str = hints
                    .iter()
                    .map(|h| format!("        // - {}", h))
                    .collect::<Vec<_>>()
                    .join("\n");

                let impl_code = format!(
                    r#"fn visit_expression(&mut self, _expr: &Expression<'a>, _source: &str) -> bool {{
        // TODO: Manual implementation required
        // This rule has a complex pattern that couldn't be auto-generated.
        //
        // Hints from Rector analysis:
{}
        //
        // Original refactor() body excerpt:
        // {}
        true
    }}"#,
                    hints_str,
                    truncate_refactor_body(refactor_body)
                );
                (impl_code, true)
            }

            RulePattern::Unknown => {
                let impl_code = r#"fn visit_expression(&mut self, _expr: &Expression<'a>, _source: &str) -> bool {
        // TODO: Manual implementation required
        // The pattern for this rule could not be detected.
        // Please refer to the original Rector rule for implementation details.
        true
    }"#.to_string();
                (impl_code, true)
            }
        }
    }

    /// Generate mod.rs for a collection of rules
    pub fn generate_mod_file(&self, rules: &[GeneratedRule]) -> Result<String, String> {
        let rule_data: Vec<_> = rules
            .iter()
            .map(|r| {
                let snake_name = r.filename.strip_suffix(".rs").unwrap_or(&r.filename);
                let pascal_name = snake_name.to_case(Case::Pascal);
                json!({
                    "snake_name": snake_name,
                    "pascal_name": pascal_name,
                })
            })
            .collect();

        let data = json!({ "rules": rule_data });

        self.handlebars
            .render("mod", &data)
            .map_err(|e| format!("Template error: {}", e))
    }

    /// Write generated rules to disk
    pub fn write_rules(&self, rules: &[GeneratedRule], output_dir: &Path) -> Result<(), String> {
        // Create output directory
        fs::create_dir_all(output_dir)
            .map_err(|e| format!("Failed to create output dir: {}", e))?;

        // Write each rule file
        for rule in rules {
            let path = output_dir.join(&rule.filename);
            fs::write(&path, &rule.source)
                .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        }

        // Generate and write mod.rs
        let mod_content = self.generate_mod_file(rules)?;
        let mod_path = output_dir.join("mod.rs");
        fs::write(&mod_path, mod_content)
            .map_err(|e| format!("Failed to write mod.rs: {}", e))?;

        Ok(())
    }
}

/// Convert Rector rule name to snake_case
fn rule_to_snake_name(name: &str) -> String {
    name.strip_suffix("Rector")
        .unwrap_or(name)
        .to_case(Case::Snake)
}

/// Convert Rector rule name to PascalCase (without Rector suffix)
fn rule_to_pascal_name(name: &str) -> String {
    name.strip_suffix("Rector")
        .unwrap_or(name)
        .to_case(Case::Pascal)
}

/// Map Rector category to rustor Category enum
fn map_category(category: &str) -> String {
    match category {
        "CodeQuality" => "Simplification",
        "DeadCode" => "Simplification",
        "Php74" => "Modernization",
        "Php80" => "Modernization",
        "Php81" => "Modernization",
        "Php82" => "Modernization",
        "Php83" => "Modernization",
        "TypeDeclaration" => "Simplification",
        "Strict" => "Compatibility",
        "Naming" => "Simplification",
        "CodingStyle" => "Simplification",
        "Performance" => "Performance",
        _ => "Simplification", // Default
    }.to_string()
}

/// Escape string for Rust string literal
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Escape code sample for doc comments - adds //! prefix to each line
fn escape_code_sample(s: &str) -> String {
    s.lines()
        .map(|line| format!("//! {}", line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Truncate refactor body for comments
fn truncate_refactor_body(body: &str) -> String {
    let lines: Vec<_> = body.lines().take(5).collect();
    if lines.len() < body.lines().count() {
        format!("{}...", lines.join("\n        // "))
    } else {
        lines.join("\n        // ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_to_snake_name() {
        assert_eq!(rule_to_snake_name("IsNullRector"), "is_null");
        assert_eq!(rule_to_snake_name("ArrayPushToShortSyntaxRector"), "array_push_to_short_syntax");
        assert_eq!(rule_to_snake_name("JoinToImplodeRector"), "join_to_implode");
    }

    #[test]
    fn test_rule_to_pascal_name() {
        assert_eq!(rule_to_pascal_name("IsNullRector"), "IsNull");
        assert_eq!(rule_to_pascal_name("ArrayPushToShortSyntaxRector"), "ArrayPushToShortSyntax");
    }

    #[test]
    fn test_map_category() {
        assert_eq!(map_category("CodeQuality"), "Simplification");
        assert_eq!(map_category("Php80"), "Modernization");
        assert_eq!(map_category("Performance"), "Performance");
    }

    #[test]
    fn test_generate_function_rename_rule() {
        let generator = CodeGenerator::new();
        let rule = RectorRule {
            name: "JoinToImplodeRector".to_string(),
            category: "CodeQuality".to_string(),
            description: "Replace join() with implode()".to_string(),
            node_types: vec!["FuncCall".to_string()],
            min_php_version: None,
            before_code: "join(',', $arr)".to_string(),
            after_code: "implode(',', $arr)".to_string(),
            pattern: RulePattern::FunctionRename {
                from: "join".to_string(),
                to: "implode".to_string(),
            },
            is_configurable: false,
            source_file: "rector/rules/CodeQuality/JoinToImplodeRector.php".to_string(),
        };

        let generated = generator.generate_rule(&rule).unwrap();
        assert_eq!(generated.filename, "join_to_implode.rs");
        assert!(!generated.needs_review);
        assert!(generated.source.contains("fn check_join_to_implode"));
        assert!(generated.source.contains("JoinToImplodeRule"));
    }

    #[test]
    fn test_generate_complex_rule_needs_review() {
        let generator = CodeGenerator::new();
        let rule = RectorRule {
            name: "ComplexRector".to_string(),
            category: "CodeQuality".to_string(),
            description: "Complex transformation".to_string(),
            node_types: vec!["FuncCall".to_string()],
            min_php_version: None,
            before_code: "// complex".to_string(),
            after_code: "// result".to_string(),
            pattern: RulePattern::Complex {
                hints: vec!["Uses type checking".to_string()],
                refactor_body: "// body".to_string(),
            },
            is_configurable: false,
            source_file: "rector/rules/Complex/ComplexRector.php".to_string(),
        };

        let generated = generator.generate_rule(&rule).unwrap();
        assert!(generated.needs_review);
        assert!(generated.source.contains("TODO: Manual implementation required"));
    }
}
