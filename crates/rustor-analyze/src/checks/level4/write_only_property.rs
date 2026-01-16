//! Check for properties that are only written to but never read (Level 4)
//!
//! Detects class properties that are assigned values but never accessed.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Checks for write-only properties
pub struct WriteOnlyPropertyCheck;

impl Check for WriteOnlyPropertyCheck {
    fn id(&self) -> &'static str {
        "property.onlyWritten"
    }

    fn description(&self) -> &'static str {
        "Detects properties that are written but never read"
    }

    fn level(&self) -> u8 {
        4
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = PropertyAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_properties: HashMap::new(),
            current_class: None,
            issues: Vec::new(),
        };

        // First pass: collect all property writes and reads
        analyzer.analyze_program(program);

        // Second pass: check for write-only properties
        analyzer.check_write_only_properties();

        analyzer.issues
    }
}

#[derive(Debug)]
struct PropertyInfo {
    name: String,
    writes: Vec<usize>,  // line numbers where property is written
    reads: Vec<usize>,   // line numbers where property is read
}

struct PropertyAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    class_properties: HashMap<String, HashMap<String, PropertyInfo>>,
    current_class: Option<String>,
    issues: Vec<Issue>,
}

impl<'s> PropertyAnalyzer<'s> {
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

    fn record_property_write(&mut self, prop_name: String, line: usize) {
        if let Some(class_name) = &self.current_class {
            let class_props = self.class_properties
                .entry(class_name.clone())
                .or_insert_with(HashMap::new);

            let prop_info = class_props
                .entry(prop_name.clone())
                .or_insert_with(|| PropertyInfo {
                    name: prop_name,
                    writes: Vec::new(),
                    reads: Vec::new(),
                });

            prop_info.writes.push(line);
        }
    }

    fn record_property_read(&mut self, prop_name: String, line: usize) {
        if let Some(class_name) = &self.current_class {
            let class_props = self.class_properties
                .entry(class_name.clone())
                .or_insert_with(HashMap::new);

            let prop_info = class_props
                .entry(prop_name.clone())
                .or_insert_with(|| PropertyInfo {
                    name: prop_name,
                    writes: Vec::new(),
                    reads: Vec::new(),
                });

            prop_info.reads.push(line);
        }
    }

    fn check_write_only_properties(&mut self) {
        for (_class_name, properties) in &self.class_properties {
            for (prop_name, prop_info) in properties {
                // Property is write-only if it has writes but no reads
                if !prop_info.writes.is_empty() && prop_info.reads.is_empty() {
                    // Report on the first write location
                    if let Some(&first_write_line) = prop_info.writes.first() {
                        self.issues.push(
                            Issue::error(
                                "property.onlyWritten",
                                format!(
                                    "Property {} is never read, only written.",
                                    prop_name
                                ),
                                self.file_path.clone(),
                                first_write_line,
                                1,
                            )
                            .with_identifier("property.onlyWritten"),
                        );
                    }
                }
            }
        }
    }

    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Class(class) => {
                // Set current class context
                let class_name = self.get_span_text(&class.name.span).to_string();
                self.current_class = Some(class_name);

                // Analyze class members
                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Method(method) => {
                            if let MethodBody::Concrete(body) = &method.body {
                                for stmt in body.statements.iter() {
                                    self.analyze_statement(stmt);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                self.current_class = None;
            }
            Statement::Expression(expr_stmt) => {
                self.analyze_expression(&expr_stmt.expression);
            }
            Statement::If(if_stmt) => {
                self.analyze_expression(&if_stmt.condition);
                self.analyze_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.analyze_expression(&while_stmt.condition);
                self.analyze_while_body(&while_stmt.body);
            }
            Statement::For(for_stmt) => {
                for init in for_stmt.initializations.iter() {
                    self.analyze_expression(init);
                }
                for cond in for_stmt.conditions.iter() {
                    self.analyze_expression(cond);
                }
                for inc in for_stmt.increments.iter() {
                    self.analyze_expression(inc);
                }
                self.analyze_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.analyze_expression(&foreach.expression);
                self.analyze_foreach_body(&foreach.body);
            }
            Statement::Function(func) => {
                for stmt in func.body.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.analyze_expression(value);
                }
            }
            Statement::Echo(echo) => {
                for value in echo.values.iter() {
                    self.analyze_expression(value);
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
            _ => {}
        }
    }

    fn analyze_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // Property write: $this->prop = value
            Expression::Assignment(assign) => {
                // Check if LHS is a property access
                let is_this_property_write = if let Expression::Access(Access::Property(prop)) = &assign.lhs {
                    if let Expression::Variable(Variable::Direct(var)) = &prop.object {
                        let var_name = self.get_span_text(&var.span);
                        if var_name == "$this" {
                            let prop_name = self.get_span_text(&prop.property.span()).to_string();
                            let (line, _) = self.get_line_col(prop.property.span().start.offset as usize);
                            self.record_property_write(prop_name, line);
                            true  // Mark that we handled this as a property write
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Only analyze LHS if it's NOT a $this->prop assignment (to avoid double-counting as read)
                if !is_this_property_write {
                    self.analyze_expression(&assign.lhs);
                }
                // Always analyze RHS
                self.analyze_expression(&assign.rhs);
            }

            // Property read: $this->prop
            Expression::Access(Access::Property(prop)) => {
                if let Expression::Variable(Variable::Direct(var)) = &prop.object {
                    let var_name = self.get_span_text(&var.span);
                    if var_name == "$this" {
                        let prop_name = self.get_span_text(&prop.property.span()).to_string();
                        let (line, _) = self.get_line_col(prop.property.span().start.offset as usize);
                        self.record_property_read(prop_name, line);
                    }
                }
                self.analyze_expression(&prop.object);
            }

            Expression::Binary(binary) => {
                self.analyze_expression(&binary.lhs);
                self.analyze_expression(&binary.rhs);
            }
            Expression::UnaryPrefix(unary) => {
                self.analyze_expression(&unary.operand);
            }
            Expression::UnaryPostfix(postfix) => {
                self.analyze_expression(&postfix.operand);
            }
            Expression::Conditional(cond) => {
                self.analyze_expression(&cond.condition);
                if let Some(then) = &cond.then {
                    self.analyze_expression(then);
                }
                self.analyze_expression(&cond.r#else);
            }
            Expression::Call(call) => {
                match call {
                    Call::Function(func_call) => {
                        for arg in func_call.argument_list.arguments.iter() {
                            self.analyze_expression(arg.value());
                        }
                    }
                    Call::Method(method_call) => {
                        self.analyze_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.analyze_expression(arg.value());
                        }
                    }
                    Call::NullSafeMethod(method_call) => {
                        self.analyze_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.analyze_expression(arg.value());
                        }
                    }
                    Call::StaticMethod(static_call) => {
                        for arg in static_call.argument_list.arguments.iter() {
                            self.analyze_expression(arg.value());
                        }
                    }
                }
            }
            Expression::Array(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.key);
                            self.analyze_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.analyze_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::LegacyArray(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.key);
                            self.analyze_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.analyze_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::Parenthesized(p) => {
                self.analyze_expression(&p.expression);
            }
            Expression::ArrayAccess(arr) => {
                self.analyze_expression(&arr.array);
                self.analyze_expression(&arr.index);
            }
            _ => {}
        }
    }

    fn analyze_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.analyze_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition);
                    self.analyze_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.analyze_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition);
                    for stmt in else_if.statements.iter() {
                        self.analyze_statement(stmt);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for stmt in else_clause.statements.iter() {
                        self.analyze_statement(stmt);
                    }
                }
            }
        }
    }

    fn analyze_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => self.analyze_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
        }
    }

    fn analyze_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => self.analyze_statement(stmt),
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
        }
    }

    fn analyze_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => self.analyze_statement(stmt),
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_only_property_check_level() {
        let check = WriteOnlyPropertyCheck;
        assert_eq!(check.level(), 4);
    }
}
