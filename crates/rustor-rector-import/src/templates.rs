//! Code generation templates for Rust rule generation
//!
//! This module provides Handlebars templates for generating rustor rules
//! from Rector rule patterns.

/// Main rule file template
pub const RULE_TEMPLATE: &str = r##"//! Rule: {{description}}
//!
//! Example:
//! ```php
//! // Before
//! {{before_code}}
//!
//! // After
//! {{after_code}}
//! ```
//!
//! Imported from Rector: {{source_file}}

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

pub fn check_{{snake_name}}<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = {{pascal_name}}Visitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct {{pascal_name}}Visitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for {{pascal_name}}Visitor<'s> {
    {{visitor_impl}}
}

pub struct {{pascal_name}}Rule;

impl Rule for {{pascal_name}}Rule {
    fn name(&self) -> &'static str {
        "{{snake_name}}"
    }

    fn description(&self) -> &'static str {
        "{{description}}"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_{{snake_name}}(program, source)
    }

    fn category(&self) -> Category {
        Category::{{category}}
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        {{php_version}}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::check_rule;

    #[test]
    fn test_basic_transformation() {
        check_rule(
            &{{pascal_name}}Rule,
            r#"<?php
{{before_code}}
"#,
            r#"<?php
{{after_code}}
"#,
        );
    }
}
"##;

/// Visitor implementation for function rename pattern
pub const VISITOR_FUNCTION_RENAME: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{{from_func}}") {
                let span = call.function.span();
                self.edits.push(Edit {
                    start: span.start.offset,
                    end: span.end.offset,
                    replacement: "{{to_func}}".to_string(),
                    message: "{{description}}".to_string(),
                });
            }
        }
        true
    }"#;

/// Visitor implementation for function alias pattern (same as rename)
pub const VISITOR_FUNCTION_ALIAS: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{{from_func}}") {
                let span = call.function.span();
                self.edits.push(Edit {
                    start: span.start.offset,
                    end: span.end.offset,
                    replacement: "{{to_func}}".to_string(),
                    message: "Replace {{from_func}}() with {{to_func}}()".to_string(),
                });
            }
        }
        true
    }"#;

/// Visitor implementation for function to comparison pattern
pub const VISITOR_FUNCTION_TO_COMPARISON: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{{func}}") {
                // Get the argument
                if let Some(arg) = call.arguments.arguments.first() {
                    let arg_str = &self.source[arg.span().start.offset..arg.span().end.offset];
                    let replacement = format!("{} {{operator}} {{compare_value}}", arg_str);

                    self.edits.push(Edit {
                        start: expr.span().start.offset,
                        end: expr.span().end.offset,
                        replacement,
                        message: "Replace {{func}}() with {{operator}} {{compare_value}} comparison".to_string(),
                    });
                }
            }
        }
        true
    }"#;

/// Visitor implementation for function to cast pattern
pub const VISITOR_FUNCTION_TO_CAST: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{{func}}") {
                // Get the argument
                if let Some(arg) = call.arguments.arguments.first() {
                    let arg_str = &self.source[arg.span().start.offset..arg.span().end.offset];
                    let replacement = format!("({{cast_type}}) {}", arg_str);

                    self.edits.push(Edit {
                        start: expr.span().start.offset,
                        end: expr.span().end.offset,
                        replacement,
                        message: "Replace {{func}}() with ({{cast_type}}) cast".to_string(),
                    });
                }
            }
        }
        true
    }"#;

/// Visitor implementation for function to operator pattern (e.g., pow -> **)
pub const VISITOR_FUNCTION_TO_OPERATOR: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{{func}}") {
                // Need exactly 2 arguments
                let args: Vec<_> = call.arguments.arguments.iter().collect();
                if args.len() == 2 {
                    let left = &self.source[args[0].span().start.offset..args[0].span().end.offset];
                    let right = &self.source[args[1].span().start.offset..args[1].span().end.offset];
                    let replacement = format!("{} {{operator}} {}", left, right);

                    self.edits.push(Edit {
                        start: expr.span().start.offset,
                        end: expr.span().end.offset,
                        replacement,
                        message: "Replace {{func}}() with {{operator}} operator".to_string(),
                    });
                }
            }
        }
        true
    }"#;

/// Visitor implementation for ternary to coalesce pattern
pub const VISITOR_TERNARY_TO_COALESCE: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Conditional(cond) = expr {
            // Check if condition is isset() or similar
            if let Expression::Call(Call::Function(call)) = &*cond.condition {
                let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

                if name_str.eq_ignore_ascii_case("{{condition_func}}") {
                    // Get the variable being checked
                    if let Some(arg) = call.arguments.arguments.first() {
                        let var_str = &self.source[arg.span().start.offset..arg.span().end.offset];

                        // Get the else value
                        if let Some(else_expr) = &cond.r#else {
                            let else_str = &self.source[else_expr.span().start.offset..else_expr.span().end.offset];
                            let replacement = format!("{} ?? {}", var_str, else_str);

                            self.edits.push(Edit {
                                start: expr.span().start.offset,
                                end: expr.span().end.offset,
                                replacement,
                                message: "Replace {{condition_func}}() ternary with ?? operator".to_string(),
                            });
                        }
                    }
                }
            }
        }
        true
    }"#;

/// Visitor implementation for function to ::class constant pattern
pub const VISITOR_FUNCTION_TO_CLASS_CONSTANT: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{{func}}") {
                // Get the argument
                if let Some(arg) = call.arguments.arguments.first() {
                    let arg_str = &self.source[arg.span().start.offset..arg.span().end.offset];
                    let replacement = format!("{}::class", arg_str);

                    self.edits.push(Edit {
                        start: expr.span().start.offset,
                        end: expr.span().end.offset,
                        replacement,
                        message: "Replace {{func}}() with ::class constant".to_string(),
                    });
                }
            }
        }
        true
    }"#;

/// Visitor implementation for function to instanceof pattern
pub const VISITOR_FUNCTION_TO_INSTANCEOF: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{{func}}") {
                // Need exactly 2 arguments: object and class name
                let args: Vec<_> = call.arguments.arguments.iter().collect();
                if args.len() >= 2 {
                    let obj_str = &self.source[args[0].span().start.offset..args[0].span().end.offset];
                    let class_str = &self.source[args[1].span().start.offset..args[1].span().end.offset];
                    // Remove ::class suffix if present
                    let class_name = class_str.trim_end_matches("::class");
                    let replacement = format!("{} instanceof {}", obj_str, class_name);

                    self.edits.push(Edit {
                        start: expr.span().start.offset,
                        end: expr.span().end.offset,
                        replacement,
                        message: "Replace {{func}}() with instanceof".to_string(),
                    });
                }
            }
        }
        true
    }"#;

/// Visitor implementation for unwrap single-arg function pattern
pub const VISITOR_UNWRAP_SINGLE_ARG: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{{func}}") {
                // Must have exactly 1 argument to unwrap
                let args: Vec<_> = call.arguments.arguments.iter().collect();
                if args.len() == 1 {
                    let arg_str = &self.source[args[0].span().start.offset..args[0].span().end.offset];

                    self.edits.push(Edit {
                        start: expr.span().start.offset,
                        end: expr.span().end.offset,
                        replacement: arg_str.to_string(),
                        message: "Remove unnecessary {{func}}() wrapper".to_string(),
                    });
                }
            }
        }
        true
    }"#;

/// Visitor implementation for function no args to function pattern
pub const VISITOR_FUNCTION_NO_ARGS: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let name_str = &self.source[call.function.span().start.offset..call.function.span().end.offset];

            if name_str.eq_ignore_ascii_case("{{from_func}}") {
                // Only match if no arguments
                if call.arguments.arguments.is_empty() {
                    self.edits.push(Edit {
                        start: expr.span().start.offset,
                        end: expr.span().end.offset,
                        replacement: "{{to_func}}()".to_string(),
                        message: "Replace {{from_func}}() with {{to_func}}()".to_string(),
                    });
                }
            }
        }
        true
    }"#;

/// Visitor implementation for nullsafe method call pattern
pub const VISITOR_NULLSAFE_METHOD_CALL: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Match ternary: $x ? $x->method() : null
        if let Expression::Conditional(cond) = expr {
            // Check if this is a nullsafe pattern
            // Pattern: $var ? $var->method() : null
            // This needs careful matching of condition variable with method call receiver
            let _ = cond; // Placeholder - complex implementation needed
        }
        true
    }"#;

/// Visitor implementation for first-class callable pattern
pub const VISITOR_FIRST_CLASS_CALLABLE: &str = r#"fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Match Closure::fromCallable([$this, 'method']) or similar
        if let Expression::Call(Call::StaticMethod(call)) = expr {
            let class_str = &self.source[call.target.span().start.offset..call.target.span().end.offset];
            let method_str = &self.source[call.method.span().start.offset..call.method.span().end.offset];

            if class_str == "Closure" && method_str == "fromCallable" {
                // Extract the callable and convert to first-class syntax
                if let Some(arg) = call.arguments.arguments.first() {
                    let arg_str = &self.source[arg.span().start.offset..arg.span().end.offset];
                    // Parse array callable like [$this, 'method'] or [self::class, 'method']
                    // This requires more complex parsing - mark for review
                    let _ = arg_str;
                }
            }
        }
        true
    }"#;

/// Visitor implementation for complex/unknown patterns (skeleton only)
pub const VISITOR_COMPLEX: &str = r#"fn visit_expression(&mut self, _expr: &Expression<'a>, _source: &str) -> bool {
        // TODO: Implement pattern detection
        // Hints from Rector analysis:
        {{#each hints}}
        // - {{this}}
        {{/each}}
        //
        // Original refactor() body:
        // {{refactor_body}}
        true
    }"#;

/// Module registration template (for mod.rs)
pub const MOD_TEMPLATE: &str = r#"//! Auto-generated rules imported from Rector
//!
//! Generated by rustor-import-rector

{{#each rules}}
pub mod {{snake_name}};
{{/each}}

use crate::registry::Rule;

/// Get all imported rules
pub fn imported_rules() -> Vec<Box<dyn Rule>> {
    vec![
        {{#each rules}}
        Box::new({{snake_name}}::{{pascal_name}}Rule),
        {{/each}}
    ]
}
"#;

/// Test file template
pub const TEST_TEMPLATE: &str = r##"//! Auto-generated tests for {{pascal_name}}Rule

use super::*;
use crate::test_utils::check_rule;

{{#each test_cases}}
#[test]
fn test_{{name}}() {
    check_rule(
        &{{../pascal_name}}Rule,
        r#"<?php
{{before}}
"#,
        r#"<?php
{{after}}
"#,
    );
}
{{/each}}
"##;

/// Get the appropriate visitor template for a rule pattern
pub fn get_visitor_template(pattern_type: &str) -> &'static str {
    match pattern_type {
        "FunctionRename" => VISITOR_FUNCTION_RENAME,
        "FunctionAlias" => VISITOR_FUNCTION_ALIAS,
        "FunctionToComparison" => VISITOR_FUNCTION_TO_COMPARISON,
        "FunctionToCast" => VISITOR_FUNCTION_TO_CAST,
        "FunctionToOperator" => VISITOR_FUNCTION_TO_OPERATOR,
        "TernaryToCoalesce" => VISITOR_TERNARY_TO_COALESCE,
        "FunctionToClassConstant" => VISITOR_FUNCTION_TO_CLASS_CONSTANT,
        "FunctionToInstanceof" => VISITOR_FUNCTION_TO_INSTANCEOF,
        "UnwrapSingleArgFunction" => VISITOR_UNWRAP_SINGLE_ARG,
        "FunctionNoArgsToFunction" => VISITOR_FUNCTION_NO_ARGS,
        "NullsafeMethodCall" => VISITOR_NULLSAFE_METHOD_CALL,
        "FirstClassCallable" => VISITOR_FIRST_CLASS_CALLABLE,
        _ => VISITOR_COMPLEX,
    }
}
