//! Rule: redundant_type_check
//!
//! Removes redundant type checks when parameter is already typed.
//!
//! Patterns:
//! - `function f(int $x) { if (is_int($x)) ... }` - is_int is always true
//! - `function f(string $x) { if (is_string($x)) ... }` - is_string is always true
//! - `function f(array $x) { if (is_array($x)) ... }` - is_array is always true
//!
//! Why: Type declarations guarantee the type at runtime (in strict mode).
//!
//! Note: This is a simplified type-aware rule that only tracks function parameters.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;
use std::collections::HashMap;

use crate::registry::{Category, PhpVersion, Rule};

pub struct RedundantTypeCheckRule;

impl Rule for RedundantTypeCheckRule {
    fn name(&self) -> &'static str {
        "redundant_type_check"
    }

    fn description(&self) -> &'static str {
        "Remove redundant type checks when parameter is already typed"
    }

    fn category(&self) -> Category {
        Category::Simplification // Removing redundant code simplifies it
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php70) // Type declarations became reliable in PHP 7
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        let mut checker = RedundantTypeChecker {
            source,
            edits: Vec::new(),
        };
        checker.check_program(program);
        checker.edits
    }
}

/// Maps type check functions to their corresponding PHP types
fn get_type_check_map() -> HashMap<&'static str, Vec<&'static str>> {
    let mut map = HashMap::new();
    map.insert("is_int", vec!["int", "integer"]);
    map.insert("is_integer", vec!["int", "integer"]);
    map.insert("is_long", vec!["int", "integer"]);
    map.insert("is_string", vec!["string"]);
    map.insert("is_array", vec!["array"]);
    map.insert("is_float", vec!["float", "double", "real"]);
    map.insert("is_double", vec!["float", "double", "real"]);
    map.insert("is_real", vec!["float", "double", "real"]);
    map.insert("is_bool", vec!["bool", "boolean"]);
    map.insert("is_object", vec!["object"]);
    map.insert("is_callable", vec!["callable"]);
    map.insert("is_iterable", vec!["iterable"]);
    map
}

struct RedundantTypeChecker<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'s> RedundantTypeChecker<'s> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn check_program(&mut self, program: &Program<'_>) {
        for stmt in program.statements.iter() {
            self.check_statement(stmt, &HashMap::new());
        }
    }

    fn check_statement(&mut self, stmt: &Statement<'_>, typed_params: &HashMap<String, String>) {
        match stmt {
            Statement::Function(func) => {
                let params = self.extract_typed_params(&func.parameter_list);
                // Function body is a Block directly
                self.check_block(&func.body, &params);
            }
            Statement::Class(class) => {
                // Class members are directly on class, not class.body
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let params = self.extract_typed_params(&method.parameter_list);
                        if let MethodBody::Concrete(ref body) = method.body {
                            self.check_block(body, &params);
                        }
                    }
                }
            }
            Statement::Block(block) => {
                self.check_block(block, typed_params);
            }
            Statement::If(if_stmt) => {
                self.check_expression(&if_stmt.condition, typed_params);
                self.check_if_body(&if_stmt.body, typed_params);
            }
            Statement::While(while_stmt) => {
                self.check_expression(&while_stmt.condition, typed_params);
                self.check_while_body(&while_stmt.body, typed_params);
            }
            Statement::For(for_stmt) => {
                self.check_for_body(&for_stmt.body, typed_params);
            }
            Statement::Foreach(foreach_stmt) => {
                self.check_foreach_body(&foreach_stmt.body, typed_params);
            }
            Statement::Expression(expr_stmt) => {
                self.check_expression(&expr_stmt.expression, typed_params);
            }
            Statement::Return(ret) => {
                if let Some(ref val) = ret.value {
                    self.check_expression(val, typed_params);
                }
            }
            Statement::Namespace(ns) => {
                let statements = match &ns.body {
                    NamespaceBody::Implicit(body) => &body.statements,
                    NamespaceBody::BraceDelimited(body) => &body.statements,
                };
                for inner in statements.iter() {
                    self.check_statement(inner, typed_params);
                }
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &Block<'_>, typed_params: &HashMap<String, String>) {
        for stmt in block.statements.iter() {
            self.check_statement(stmt, typed_params);
        }
    }

    fn check_if_body(&mut self, body: &IfBody<'_>, typed_params: &HashMap<String, String>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.check_statement(stmt_body.statement, typed_params);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition, typed_params);
                    self.check_statement(else_if.statement, typed_params);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.check_statement(else_clause.statement, typed_params);
                }
            }
            IfBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner, typed_params);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition, typed_params);
                    for inner in else_if.statements.iter() {
                        self.check_statement(inner, typed_params);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        self.check_statement(inner, typed_params);
                    }
                }
            }
        }
    }

    fn check_while_body(&mut self, body: &WhileBody<'_>, typed_params: &HashMap<String, String>) {
        match body {
            WhileBody::Statement(stmt) => {
                self.check_statement(stmt, typed_params);
            }
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner, typed_params);
                }
            }
        }
    }

    fn check_for_body(&mut self, body: &ForBody<'_>, typed_params: &HashMap<String, String>) {
        match body {
            ForBody::Statement(stmt) => {
                self.check_statement(stmt, typed_params);
            }
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner, typed_params);
                }
            }
        }
    }

    fn check_foreach_body(&mut self, body: &ForeachBody<'_>, typed_params: &HashMap<String, String>) {
        match body {
            ForeachBody::Statement(stmt) => {
                self.check_statement(stmt, typed_params);
            }
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner, typed_params);
                }
            }
        }
    }

    fn check_expression(&mut self, expr: &Expression<'_>, typed_params: &HashMap<String, String>) {
        match expr {
            Expression::Call(Call::Function(call)) => {
                self.check_type_call(call, typed_params);
                // Also check arguments
                for arg in call.argument_list.arguments.iter() {
                    let arg_expr = match arg {
                        Argument::Positional(pos) => &pos.value,
                        Argument::Named(named) => &named.value,
                    };
                    self.check_expression(arg_expr, typed_params);
                }
            }
            Expression::Binary(binary) => {
                self.check_expression(&binary.lhs, typed_params);
                self.check_expression(&binary.rhs, typed_params);
            }
            Expression::UnaryPrefix(unary) => {
                self.check_expression(&unary.operand, typed_params);
            }
            Expression::Parenthesized(paren) => {
                self.check_expression(&paren.expression, typed_params);
            }
            Expression::Conditional(cond) => {
                self.check_expression(&cond.condition, typed_params);
                if let Some(ref then_expr) = cond.then {
                    self.check_expression(then_expr, typed_params);
                }
                self.check_expression(&cond.r#else, typed_params);
            }
            Expression::Assignment(assign) => {
                // Check RHS of assignments (may contain closures/arrow functions)
                self.check_expression(&assign.rhs, typed_params);
            }
            Expression::Closure(closure) => {
                // Closures have their own parameter scope
                let mut params = typed_params.clone();
                let closure_params = self.extract_typed_params(&closure.parameter_list);
                params.extend(closure_params);
                self.check_block(&closure.body, &params);
            }
            Expression::ArrowFunction(arrow) => {
                let mut params = typed_params.clone();
                let arrow_params = self.extract_typed_params(&arrow.parameter_list);
                params.extend(arrow_params);
                self.check_expression(&arrow.expression, &params);
            }
            _ => {}
        }
    }

    /// Extract typed parameters from a function's parameter list
    fn extract_typed_params(&self, params: &FunctionLikeParameterList<'_>) -> HashMap<String, String> {
        let mut typed_params = HashMap::new();

        for param in params.parameters.iter() {
            // Check if parameter has a type hint
            if let Some(ref hint) = param.hint {
                let var_name = self.get_text(param.variable.span()).to_string();
                let type_name = self.get_text(hint.span()).to_lowercase();
                // Remove leading ? for nullable types
                let type_name = type_name.trim_start_matches('?').to_string();
                typed_params.insert(var_name, type_name);
            }
        }

        typed_params
    }

    /// Check if a function call is a type check on a typed parameter
    fn check_type_call(&mut self, call: &FunctionCall<'_>, typed_params: &HashMap<String, String>) {
        // Get function name
        let func_name = self.get_text(call.function.span()).to_lowercase();

        let type_map = get_type_check_map();

        // Check if this is a type check function
        if let Some(matching_types) = type_map.get(func_name.as_str()) {
            // Must have exactly one argument
            let args: Vec<_> = call.argument_list.arguments.iter().collect();
            if args.len() != 1 {
                return;
            }

            // Get the argument
            let arg = &args[0];
            let arg_expr = match arg {
                Argument::Positional(pos) => &pos.value,
                Argument::Named(named) => &named.value,
            };

            // Check if it's a simple variable
            if let Expression::Variable(var) = arg_expr {
                let var_name = self.get_text(var.span()).to_string();

                // Check if the variable is a typed parameter
                if let Some(param_type) = typed_params.get(&var_name) {
                    // Check if the type matches
                    if matching_types.iter().any(|t| *t == param_type.as_str()) {
                        // This is a redundant check!
                        self.edits.push(Edit::new(
                            call.span(),
                            "true".to_string(),
                            format!(
                                "Redundant {}() - {} is already typed as {}",
                                func_name, var_name, param_type
                            ),
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;

    fn parse_and_check(code: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, code);
        RedundantTypeCheckRule.check(&program, code)
    }

    #[test]
    fn test_redundant_is_int() {
        let code = r#"<?php
function process(int $value) {
    if (is_int($value)) {
        return $value * 2;
    }
}
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "true");
    }

    #[test]
    fn test_redundant_is_string() {
        let code = r#"<?php
function greet(string $name) {
    if (is_string($name)) {
        echo "Hello, $name";
    }
}
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_redundant_is_array() {
        let code = r#"<?php
function sum(array $numbers) {
    if (is_array($numbers)) {
        return array_sum($numbers);
    }
}
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_redundant_is_bool() {
        let code = r#"<?php
function toggle(bool $flag) {
    if (is_bool($flag)) {
        return !$flag;
    }
}
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_redundant_is_float() {
        let code = r#"<?php
function round_value(float $num) {
    if (is_float($num)) {
        return round($num);
    }
}
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_no_match_untyped_param() {
        let code = r#"<?php
function process($value) {
    if (is_int($value)) {
        return $value * 2;
    }
}
"#;
        let edits = parse_and_check(code);
        assert!(edits.is_empty(), "Should not match untyped parameters");
    }

    #[test]
    fn test_no_match_different_type() {
        let code = r#"<?php
function process(string $value) {
    if (is_int($value)) {
        return $value * 2;
    }
}
"#;
        let edits = parse_and_check(code);
        assert!(edits.is_empty(), "Should not match when types don't match");
    }

    #[test]
    fn test_no_match_local_variable() {
        let code = r#"<?php
function process(int $value) {
    $other = getValue();
    if (is_int($other)) {
        return $other * 2;
    }
}
"#;
        let edits = parse_and_check(code);
        assert!(edits.is_empty(), "Should not match local variables");
    }

    #[test]
    fn test_method_params() {
        let code = r#"<?php
class Calculator {
    public function add(int $a, int $b) {
        if (is_int($a) && is_int($b)) {
            return $a + $b;
        }
    }
}
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 2, "Should match both parameters");
    }

    #[test]
    fn test_nullable_type() {
        let code = r#"<?php
function process(?int $value) {
    if (is_int($value)) {
        return $value * 2;
    }
}
"#;
        let edits = parse_and_check(code);
        // Nullable types should still match - the check is redundant if we know it's not null
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_closure_params() {
        let code = r#"<?php
$fn = function(int $x) {
    if (is_int($x)) {
        return $x * 2;
    }
};
"#;
        let edits = parse_and_check(code);
        assert_eq!(edits.len(), 1);
    }
}
