//! Check for unused constructor parameters (Level 1)
//!
//! PHPStan reports unused constructor parameters at level 1.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashSet;
use std::path::PathBuf;

/// Checks for unused constructor parameters
pub struct UnusedConstructorParameterCheck;

impl Check for UnusedConstructorParameterCheck {
    fn id(&self) -> &'static str {
        "constructor.unusedParameter"
    }

    fn description(&self) -> &'static str {
        "Detects unused constructor parameters"
    }

    fn level(&self) -> u8 {
        1
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = UnusedParamAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };

        analyzer.analyze_program(program);
        analyzer.issues
    }
}

struct UnusedParamAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

impl<'s> UnusedParamAnalyzer<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn get_line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span);
                        if method_name.eq_ignore_ascii_case("__construct") {
                            self.check_constructor(method, &class_name);
                        }
                    }
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            },
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            _ => {}
        }
    }

    fn check_constructor<'a>(&mut self, method: &Method<'a>, class_name: &str) {
        let body = match &method.body {
            MethodBody::Concrete(body) => body,
            MethodBody::Abstract(_) => return,
        };

        // Collect parameter names (excluding promoted properties)
        let mut param_names: Vec<(String, mago_span::Span)> = Vec::new();
        for param in method.parameter_list.parameters.iter() {
            // Check if it's a promoted property (has visibility modifier)
            let is_promoted = param.modifiers.iter().any(|m| {
                matches!(
                    m,
                    Modifier::Public(_) | Modifier::Protected(_) | Modifier::Private(_)
                )
            });

            if !is_promoted {
                let name = self.get_span_text(&param.variable.span());
                // Remove the $ prefix for comparison
                let name_without_dollar = name.trim_start_matches('$').to_string();
                param_names.push((name_without_dollar, param.variable.span()));
            }
        }

        if param_names.is_empty() {
            return;
        }

        // Find all variable usages in the constructor body
        let mut used_vars: HashSet<String> = HashSet::new();
        self.collect_used_variables_in_block(body, &mut used_vars);

        // Report unused parameters
        for (param_name, span) in param_names {
            if !used_vars.contains(&param_name) {
                let (line, col) = self.get_line_col(span.start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "constructor.unusedParameter",
                        format!(
                            "Constructor of class {} has an unused parameter ${}.",
                            class_name, param_name
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("constructor.unusedParameter"),
                );
            }
        }
    }

    fn collect_used_variables_in_block<'a>(&self, block: &Block<'a>, used: &mut HashSet<String>) {
        for stmt in block.statements.iter() {
            self.collect_from_statement(stmt, used);
        }
    }

    fn collect_from_statement<'a>(&self, stmt: &Statement<'a>, used: &mut HashSet<String>) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.collect_from_expression(&expr_stmt.expression, used);
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.collect_from_expression(value, used);
                }
            }
            Statement::If(if_stmt) => {
                self.collect_from_expression(&if_stmt.condition, used);
                self.collect_from_if_body(&if_stmt.body, used);
            }
            Statement::While(while_stmt) => {
                self.collect_from_expression(&while_stmt.condition, used);
                self.collect_from_while_body(&while_stmt.body, used);
            }
            Statement::For(for_stmt) => {
                for init in for_stmt.initializations.iter() {
                    self.collect_from_expression(init, used);
                }
                for cond in for_stmt.conditions.iter() {
                    self.collect_from_expression(cond, used);
                }
                for inc in for_stmt.increments.iter() {
                    self.collect_from_expression(inc, used);
                }
                self.collect_from_for_body(&for_stmt.body, used);
            }
            Statement::Foreach(foreach) => {
                self.collect_from_expression(&foreach.expression, used);
                self.collect_from_foreach_body(&foreach.body, used);
            }
            Statement::Block(block) => {
                self.collect_used_variables_in_block(block, used);
            }
            Statement::Try(try_stmt) => {
                self.collect_used_variables_in_block(&try_stmt.block, used);
                for catch in try_stmt.catch_clauses.iter() {
                    self.collect_used_variables_in_block(&catch.block, used);
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    self.collect_used_variables_in_block(&finally.block, used);
                }
            }
            Statement::Switch(switch) => {
                self.collect_from_expression(&switch.expression, used);
                self.collect_from_switch_body(&switch.body, used);
            }
            Statement::Echo(echo) => {
                for expr in echo.values.iter() {
                    self.collect_from_expression(expr, used);
                }
            }
            _ => {}
        }
    }

    fn collect_from_expression<'a>(&self, expr: &Expression<'a>, used: &mut HashSet<String>) {
        match expr {
            Expression::Variable(var) => {
                let name = self.get_span_text(&var.span());
                let name = name.trim_start_matches('$');
                used.insert(name.to_string());
            }
            Expression::Assignment(assign) => {
                self.collect_from_expression(&assign.rhs, used);
                self.collect_from_expression(&assign.lhs, used);
            }
            Expression::Binary(binary) => {
                self.collect_from_expression(&binary.lhs, used);
                self.collect_from_expression(&binary.rhs, used);
            }
            Expression::UnaryPrefix(unary) => {
                self.collect_from_expression(&unary.operand, used);
            }
            Expression::UnaryPostfix(unary) => {
                self.collect_from_expression(&unary.operand, used);
            }
            Expression::Parenthesized(paren) => {
                self.collect_from_expression(&paren.expression, used);
            }
            Expression::Call(call) => {
                match call {
                    Call::Function(func_call) => {
                        self.collect_from_expression(&func_call.function, used);
                        for arg in func_call.argument_list.arguments.iter() {
                            self.collect_from_expression(arg.value(), used);
                        }
                    }
                    Call::Method(method_call) => {
                        self.collect_from_expression(&method_call.object, used);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.collect_from_expression(arg.value(), used);
                        }
                    }
                    Call::StaticMethod(static_call) => {
                        for arg in static_call.argument_list.arguments.iter() {
                            self.collect_from_expression(arg.value(), used);
                        }
                    }
                    Call::NullSafeMethod(ns_call) => {
                        self.collect_from_expression(&ns_call.object, used);
                        for arg in ns_call.argument_list.arguments.iter() {
                            self.collect_from_expression(arg.value(), used);
                        }
                    }
                }
            }
            Expression::Access(access) => {
                match access {
                    Access::Property(prop) => {
                        self.collect_from_expression(&prop.object, used);
                    }
                    Access::NullSafeProperty(nsp) => {
                        self.collect_from_expression(&nsp.object, used);
                    }
                    Access::StaticProperty(sp) => {
                        let name = self.get_span_text(&sp.property.span());
                        let name = name.trim_start_matches('$');
                        used.insert(name.to_string());
                    }
                    Access::ClassConstant(_) => {}
                }
            }
            Expression::ArrayAccess(arr) => {
                self.collect_from_expression(&arr.array, used);
                self.collect_from_expression(&arr.index, used);
            }
            Expression::Array(arr) => {
                for item in arr.elements.iter() {
                    match item {
                        ArrayElement::KeyValue(kv) => {
                            self.collect_from_expression(&kv.key, used);
                            self.collect_from_expression(&kv.value, used);
                        }
                        ArrayElement::Value(val) => {
                            self.collect_from_expression(&val.value, used);
                        }
                        ArrayElement::Variadic(var) => {
                            self.collect_from_expression(&var.value, used);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::LegacyArray(arr) => {
                for item in arr.elements.iter() {
                    match item {
                        ArrayElement::KeyValue(kv) => {
                            self.collect_from_expression(&kv.key, used);
                            self.collect_from_expression(&kv.value, used);
                        }
                        ArrayElement::Value(val) => {
                            self.collect_from_expression(&val.value, used);
                        }
                        ArrayElement::Variadic(var) => {
                            self.collect_from_expression(&var.value, used);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::Conditional(tern) => {
                self.collect_from_expression(&tern.condition, used);
                if let Some(then) = &tern.then {
                    self.collect_from_expression(then, used);
                }
                self.collect_from_expression(&tern.r#else, used);
            }
            Expression::Instantiation(inst) => {
                if let Some(args) = &inst.argument_list {
                    for arg in args.arguments.iter() {
                        self.collect_from_expression(arg.value(), used);
                    }
                }
            }
            Expression::Closure(closure) => {
                // Check use clause for variable bindings
                if let Some(use_clause) = &closure.use_clause {
                    for var in use_clause.variables.iter() {
                        let name = self.get_span_text(&var.variable.span());
                        let name = name.trim_start_matches('$');
                        used.insert(name.to_string());
                    }
                }
            }
            Expression::ArrowFunction(arrow) => {
                self.collect_from_expression(&arrow.expression, used);
            }
            Expression::Match(m) => {
                self.collect_from_expression(&m.expression, used);
                for arm in m.arms.iter() {
                    match arm {
                        MatchArm::Expression(arm_expr) => {
                            for cond in arm_expr.conditions.iter() {
                                self.collect_from_expression(cond, used);
                            }
                            self.collect_from_expression(&arm_expr.expression, used);
                        }
                        MatchArm::Default(arm_default) => {
                            self.collect_from_expression(&arm_default.expression, used);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_from_if_body<'a>(&self, body: &IfBody<'a>, used: &mut HashSet<String>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.collect_from_statement(stmt_body.statement, used);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.collect_from_expression(&else_if.condition, used);
                    self.collect_from_statement(else_if.statement, used);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.collect_from_statement(else_clause.statement, used);
                }
            }
            IfBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.collect_from_statement(stmt, used);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.collect_from_expression(&else_if.condition, used);
                    for stmt in else_if.statements.iter() {
                        self.collect_from_statement(stmt, used);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for stmt in else_clause.statements.iter() {
                        self.collect_from_statement(stmt, used);
                    }
                }
            }
        }
    }

    fn collect_from_while_body<'a>(&self, body: &WhileBody<'a>, used: &mut HashSet<String>) {
        match body {
            WhileBody::Statement(stmt) => self.collect_from_statement(stmt, used),
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.collect_from_statement(stmt, used);
                }
            }
        }
    }

    fn collect_from_for_body<'a>(&self, body: &ForBody<'a>, used: &mut HashSet<String>) {
        match body {
            ForBody::Statement(stmt) => self.collect_from_statement(stmt, used),
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.collect_from_statement(stmt, used);
                }
            }
        }
    }

    fn collect_from_foreach_body<'a>(&self, body: &ForeachBody<'a>, used: &mut HashSet<String>) {
        match body {
            ForeachBody::Statement(stmt) => self.collect_from_statement(stmt, used),
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.collect_from_statement(stmt, used);
                }
            }
        }
    }

    fn collect_from_switch_body<'a>(&self, body: &SwitchBody<'a>, used: &mut HashSet<String>) {
        match body {
            SwitchBody::BraceDelimited(block) => {
                for case in block.cases.iter() {
                    match case {
                        SwitchCase::Expression(c) => {
                            self.collect_from_expression(&c.expression, used);
                            for stmt in c.statements.iter() {
                                self.collect_from_statement(stmt, used);
                            }
                        }
                        SwitchCase::Default(d) => {
                            for stmt in d.statements.iter() {
                                self.collect_from_statement(stmt, used);
                            }
                        }
                    }
                }
            }
            SwitchBody::ColonDelimited(block) => {
                for case in block.cases.iter() {
                    match case {
                        SwitchCase::Expression(c) => {
                            self.collect_from_expression(&c.expression, used);
                            for stmt in c.statements.iter() {
                                self.collect_from_statement(stmt, used);
                            }
                        }
                        SwitchCase::Default(d) => {
                            for stmt in d.statements.iter() {
                                self.collect_from_statement(stmt, used);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unused_param_check_level() {
        let check = UnusedConstructorParameterCheck;
        assert_eq!(check.level(), 1);
    }
}
