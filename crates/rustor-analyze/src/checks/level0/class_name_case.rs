//! Check for incorrect class name casing (Level 0)
//!
//! Detects when class names are referenced with incorrect case.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Checks for class name case mismatches
pub struct ClassNameCaseCheck;

impl Check for ClassNameCaseCheck {
    fn id(&self) -> &'static str {
        "class.nameCase"
    }

    fn description(&self) -> &'static str {
        "Detects class name references with incorrect casing"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = ClassCaseAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_declarations: HashMap::new(),
            symbol_table: ctx.symbol_table,
            issues: Vec::new(),
        };

        // First pass: collect class declarations
        analyzer.collect_declarations(program);

        // Second pass: check references
        analyzer.check_references(program);

        analyzer.issues
    }
}

struct ClassCaseAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    class_declarations: HashMap<String, String>, // lowercase -> correct case
    symbol_table: Option<&'s crate::symbols::SymbolTable>,
    issues: Vec<Issue>,
}

impl<'s> ClassCaseAnalyzer<'s> {
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

    /// Get the correct case for a class name
    fn get_correct_case(&self, class_name: &str) -> Option<&str> {
        let class_lower = class_name.to_lowercase();

        // Check local declarations first
        if let Some(correct) = self.class_declarations.get(&class_lower) {
            return Some(correct);
        }

        // Check symbol table
        if let Some(symbol_table) = self.symbol_table {
            if let Some(class_info) = symbol_table.get_class(class_name) {
                return Some(&class_info.name);
            }
        }

        None
    }

    /// Check if a class name has incorrect casing
    fn check_class_name(&mut self, used_name: &str, span: &mago_span::Span) {
        // Skip built-in classes and keywords
        let used_lower = used_name.to_lowercase();
        if matches!(used_lower.as_str(),
            "self" | "parent" | "static" |
            "true" | "false" | "null" |
            "int" | "float" | "string" | "bool" | "array" | "object" | "mixed" | "void" | "callable"
        ) {
            return;
        }

        if let Some(correct_case) = self.get_correct_case(used_name) {
            if used_name != correct_case {
                let (line, col) = self.get_line_col(span.start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "class.nameCase",
                        format!(
                            "Class name {} is referenced with incorrect case, should be {}.",
                            used_name, correct_case
                        ),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("class.nameCase"),
                );
            }
        }
    }

    fn collect_declarations(&mut self, program: &Program) {
        for stmt in program.statements.iter() {
            self.collect_from_statement(stmt);
        }
    }

    fn collect_from_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();
                let class_lower = class_name.to_lowercase();
                self.class_declarations.insert(class_lower, class_name);
            }
            Statement::Interface(interface) => {
                let name = self.get_span_text(&interface.name.span).to_string();
                let name_lower = name.to_lowercase();
                self.class_declarations.insert(name_lower, name);
            }
            Statement::Trait(trait_def) => {
                let name = self.get_span_text(&trait_def.name.span).to_string();
                let name_lower = name.to_lowercase();
                self.class_declarations.insert(name_lower, name);
            }
            Statement::Enum(enum_def) => {
                let name = self.get_span_text(&enum_def.name.span).to_string();
                let name_lower = name.to_lowercase();
                self.class_declarations.insert(name_lower, name);
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_statement(inner);
                    }
                }
            },
            _ => {}
        }
    }

    fn check_references(&mut self, program: &Program) {
        for stmt in program.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                // Check extends clause
                if let Some(extends) = &class.extends {
                    for parent in extends.types.iter() {
                        let parent_name = self.get_span_text(&parent.span()).to_string();
                        let parent_span = parent.span();
                        self.check_class_name(&parent_name, &parent_span);
                    }
                }

                // Check implements clause
                if let Some(implements) = &class.implements {
                    for interface in implements.types.iter() {
                        let interface_name = self.get_span_text(&interface.span()).to_string();
                        let interface_span = interface.span();
                        self.check_class_name(&interface_name, &interface_span);
                    }
                }

                // Check class members
                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Method(method) => {
                            if let MethodBody::Concrete(body) = &method.body {
                                for stmt in body.statements.iter() {
                                    self.check_statement(stmt);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::Function(func) => {
                for stmt in func.body.statements.iter() {
                    self.check_statement(stmt);
                }
            }
            Statement::Expression(expr_stmt) => {
                self.check_expression(&expr_stmt.expression);
            }
            Statement::If(if_stmt) => {
                self.check_expression(&if_stmt.condition);
                self.check_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.check_expression(&while_stmt.condition);
                self.check_while_body(&while_stmt.body);
            }
            Statement::For(for_stmt) => {
                for init in for_stmt.initializations.iter() {
                    self.check_expression(init);
                }
                for cond in for_stmt.conditions.iter() {
                    self.check_expression(cond);
                }
                for inc in for_stmt.increments.iter() {
                    self.check_expression(inc);
                }
                self.check_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.check_expression(&foreach.expression);
                self.check_foreach_body(&foreach.body);
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner);
                }
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.check_expression(value);
                }
            }
            Statement::Echo(echo) => {
                for value in echo.values.iter() {
                    self.check_expression(value);
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.check_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.check_statement(inner);
                    }
                }
            },
            _ => {}
        }
    }

    fn check_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // Check instantiation: new ClassName()
            Expression::Instantiation(inst) => {
                if let Expression::Identifier(ident) = &*inst.class {
                    let class_name = self.get_span_text(&ident.span()).to_string();
                    let span = ident.span();
                    self.check_class_name(&class_name, &span);
                }
            }

            // Check static calls: ClassName::method()
            Expression::Call(Call::StaticMethod(static_call)) => {
                if let Expression::Identifier(ident) = &*static_call.class {
                    let class_name = self.get_span_text(&ident.span()).to_string();
                    let span = ident.span();
                    self.check_class_name(&class_name, &span);
                }
                for arg in static_call.argument_list.arguments.iter() {
                    self.check_expression(arg.value());
                }
            }

            // Check instanceof: $x instanceof ClassName
            Expression::Binary(binary) if matches!(binary.operator, BinaryOperator::Instanceof(_)) => {
                self.check_expression(&binary.lhs);
                if let Expression::Identifier(ident) = &*binary.rhs {
                    let class_name = self.get_span_text(&ident.span()).to_string();
                    let span = ident.span();
                    self.check_class_name(&class_name, &span);
                }
            }

            Expression::Binary(binary) => {
                self.check_expression(&binary.lhs);
                self.check_expression(&binary.rhs);
            }
            Expression::UnaryPrefix(unary) => {
                self.check_expression(&unary.operand);
            }
            Expression::UnaryPostfix(postfix) => {
                self.check_expression(&postfix.operand);
            }
            Expression::Conditional(cond) => {
                self.check_expression(&cond.condition);
                if let Some(then) = &cond.then {
                    self.check_expression(then);
                }
                self.check_expression(&cond.r#else);
            }
            Expression::Assignment(assign) => {
                self.check_expression(&assign.lhs);
                self.check_expression(&assign.rhs);
            }
            Expression::Call(call) => {
                match call {
                    Call::Function(func_call) => {
                        for arg in func_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                    Call::Method(method_call) => {
                        self.check_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                    Call::NullSafeMethod(method_call) => {
                        self.check_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                    Call::StaticMethod(_) => {
                        // Already handled above
                    }
                }
            }
            Expression::Array(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.check_expression(&kv.key);
                            self.check_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.check_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.check_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::LegacyArray(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.check_expression(&kv.key);
                            self.check_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.check_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.check_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::Parenthesized(p) => {
                self.check_expression(&p.expression);
            }
            Expression::ArrayAccess(arr) => {
                self.check_expression(&arr.array);
                self.check_expression(&arr.index);
            }
            _ => {}
        }
    }

    fn check_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.check_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                    self.check_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.check_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                    for stmt in else_if.statements.iter() {
                        self.check_statement(stmt);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for stmt in else_clause.statements.iter() {
                        self.check_statement(stmt);
                    }
                }
            }
        }
    }

    fn check_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => self.check_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => self.check_statement(stmt),
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => self.check_statement(stmt),
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_name_case_check_level() {
        let check = ClassNameCaseCheck;
        assert_eq!(check.level(), 0);
    }
}
