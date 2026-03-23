//! Check for duplicate keys in array literals
//!
//! - `array.duplicateKey`: Detects duplicate keys in array expressions

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Detects duplicate keys in array literals (both `[]` and `array()` syntax)
pub struct DuplicateArrayKeyCheck;

impl Check for DuplicateArrayKeyCheck {
    fn id(&self) -> &'static str {
        "array.duplicateKey"
    }

    fn description(&self) -> &'static str {
        "Detects duplicate keys in array literals"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut analyzer = DuplicateArrayKeyAnalyzer {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
        };
        analyzer.analyze_program(program);
        analyzer.issues
    }
}

struct DuplicateArrayKeyAnalyzer<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
}

impl<'s> DuplicateArrayKeyAnalyzer<'s> {
    fn line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1usize;
        let mut col = 1usize;
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

    fn span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Extract a normalized key string from an array key expression.
    /// Returns None if the key is not a static literal.
    fn extract_key(&self, expr: &Expression<'_>) -> Option<String> {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::String(s) => {
                    let text = self.span_text(&s.span());
                    // Strip quotes
                    let inner = if (text.starts_with('\'') && text.ends_with('\''))
                        || (text.starts_with('"') && text.ends_with('"'))
                    {
                        &text[1..text.len() - 1]
                    } else {
                        text
                    };
                    Some(inner.to_string())
                }
                Literal::Integer(i) => {
                    let text = self.span_text(&i.span());
                    Some(text.to_string())
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Check array elements for duplicate keys, accepting any iterable of ArrayElement
    fn check_array_elements<'a, I>(&mut self, elements: I)
    where
        I: Iterator<Item = &'a ArrayElement<'a>>,
    {
        // Map from normalized key -> (display_key, count, first_offset)
        let mut seen: HashMap<String, (String, usize, usize)> = HashMap::new();

        for element in elements {
            if let ArrayElement::KeyValue(kv) = element {
                if let Some(key) = self.extract_key(&kv.key) {
                    let offset = kv.key.span().start.offset as usize;
                    let entry = seen.entry(key.clone()).or_insert_with(|| {
                        (key.clone(), 0, offset)
                    });
                    entry.1 += 1;
                }
            }
        }

        for (_key, (display_key, count, offset)) in &seen {
            if *count > 1 {
                let (line, col) = self.line_col(*offset);
                self.issues.push(
                    Issue::error(
                        "array.duplicateKey",
                        {
                            // PHPStan format: "Array has N duplicate keys with value 'x' ('x', 'x')."
                            let repeated: Vec<String> = (0..*count).map(|_| format!("'{}'", display_key)).collect();
                            format!(
                                "Array has {} duplicate keys with value '{}' ({}).",
                                count, display_key, repeated.join(", ")
                            )
                        },
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("array.duplicateKey"),
                );
            }
        }
    }

    fn analyze_program(&mut self, program: &Program<'_>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.analyze_expression(&expr_stmt.expression);
            }
            Statement::Return(ret) => {
                if let Some(expr) = &ret.value {
                    self.analyze_expression(expr);
                }
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Method(method) => {
                            if let MethodBody::Concrete(body) = &method.body {
                                for inner in body.statements.iter() {
                                    self.analyze_statement(inner);
                                }
                            }
                        }
                        ClassLikeMember::Constant(constant) => {
                            for item in constant.items.iter() {
                                self.analyze_expression(&item.value);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::Function(func) => {
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
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
            Statement::If(if_stmt) => {
                self.analyze_if_body(&if_stmt.body);
            }
            Statement::While(w) => {
                self.analyze_while_body(&w.body);
            }
            Statement::For(f) => {
                self.analyze_for_body(&f.body);
            }
            Statement::Foreach(fe) => {
                self.analyze_foreach_body(&fe.body);
            }
            Statement::Try(t) => {
                for inner in t.block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for catch in t.catch_clauses.iter() {
                    for inner in catch.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                if let Some(finally) = &t.finally_clause {
                    for inner in finally.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            }
            Statement::Switch(sw) => {
                self.analyze_switch(sw);
            }
            _ => {}
        }
    }

    fn analyze_expression(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::Array(arr) => {
                self.check_array_elements(arr.elements.iter());
                // Also recurse into values
                for element in arr.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value);
                        }
                        _ => {}
                    }
                }
            }
            Expression::LegacyArray(arr) => {
                self.check_array_elements(arr.elements.iter());
                for element in arr.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.analyze_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.analyze_expression(&val.value);
                        }
                        _ => {}
                    }
                }
            }
            Expression::Parenthesized(p) => {
                self.analyze_expression(&p.expression);
            }
            Expression::Assignment(assign) => {
                self.analyze_expression(&assign.rhs);
            }
            Expression::Call(call) => {
                match call {
                    Call::Function(fc) => {
                        for arg in fc.argument_list.arguments.iter() {
                            self.analyze_expression(&arg.value());
                        }
                    }
                    Call::Method(mc) => {
                        for arg in mc.argument_list.arguments.iter() {
                            self.analyze_expression(&arg.value());
                        }
                    }
                    Call::StaticMethod(smc) => {
                        for arg in smc.argument_list.arguments.iter() {
                            self.analyze_expression(&arg.value());
                        }
                    }
                    _ => {}
                }
            }
            Expression::Instantiation(inst) => {
                if let Some(args) = &inst.argument_list {
                    for arg in args.arguments.iter() {
                        self.analyze_expression(&arg.value());
                    }
                }
            }
            Expression::Match(m) => {
                self.analyze_expression(&m.expression);
                for arm in m.arms.iter() {
                    match arm {
                        MatchArm::Expression(expr_arm) => {
                            self.analyze_expression(&expr_arm.expression);
                        }
                        MatchArm::Default(def_arm) => {
                            self.analyze_expression(&def_arm.expression);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn analyze_if_body(&mut self, body: &IfBody<'_>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.analyze_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.analyze_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.analyze_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for else_if in block.else_if_clauses.iter() {
                    for inner in else_if.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            }
        }
    }

    fn analyze_while_body(&mut self, body: &WhileBody<'_>) {
        match body {
            WhileBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_for_body(&mut self, body: &ForBody<'_>) {
        match body {
            ForBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_foreach_body(&mut self, body: &ForeachBody<'_>) {
        match body {
            ForeachBody::Statement(stmt) => {
                self.analyze_statement(stmt);
            }
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_switch(&mut self, switch: &Switch<'_>) {
        match &switch.body {
            SwitchBody::BraceDelimited(b) => {
                for case in b.cases.iter() {
                    let stmts: Vec<_> = match case {
                        SwitchCase::Expression(c) => c.statements.iter().collect(),
                        SwitchCase::Default(d) => d.statements.iter().collect(),
                    };
                    for stmt in stmts {
                        self.analyze_statement(stmt);
                    }
                }
            }
            SwitchBody::ColonDelimited(b) => {
                for case in b.cases.iter() {
                    let stmts: Vec<_> = match case {
                        SwitchCase::Expression(c) => c.statements.iter().collect(),
                        SwitchCase::Default(d) => d.statements.iter().collect(),
                    };
                    for stmt in stmts {
                        self.analyze_statement(stmt);
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

    fn run_check(code: &str) -> Vec<Issue> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, code);
        let config = crate::config::PhpStanConfig::default();
        let ctx = CheckContext {
            file_path: std::path::Path::new("test.php"),
            source: code,
            config: &config,
            builtin_functions: &[],
            builtin_classes: &[],
            symbol_table: None,
            scope: None,
            analysis_level: 0,
        };
        DuplicateArrayKeyCheck.check(program, &ctx)
    }

    #[test]
    fn test_duplicate_array_key_check_metadata() {
        assert_eq!(DuplicateArrayKeyCheck.level(), 0);
        assert_eq!(DuplicateArrayKeyCheck.id(), "array.duplicateKey");
    }

    #[test]
    fn test_no_duplicate_keys() {
        let code = r#"<?php
$a = ['foo' => 1, 'bar' => 2, 'baz' => 3];
"#;
        let issues = run_check(code);
        assert!(issues.is_empty(), "Expected no issues, got {:?}", issues);
    }

    #[test]
    fn test_duplicate_string_keys() {
        let code = r#"<?php
$a = ['foo' => 1, 'bar' => 2, 'foo' => 3];
"#;
        let issues = run_check(code);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("foo"));
        assert!(issues[0].message.contains("duplicate"));
    }

    #[test]
    fn test_duplicate_integer_keys() {
        let code = r#"<?php
$a = [1 => 'a', 2 => 'b', 1 => 'c'];
"#;
        let issues = run_check(code);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("1"));
    }

    #[test]
    fn test_legacy_array_syntax() {
        let code = r#"<?php
$a = array('x' => 1, 'y' => 2, 'x' => 3);
"#;
        let issues = run_check(code);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("x"));
    }

    #[test]
    fn test_no_keys_no_duplicates() {
        let code = r#"<?php
$a = [1, 2, 3];
"#;
        let issues = run_check(code);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_nested_array_duplicates() {
        let code = r#"<?php
$a = ['outer' => ['inner' => 1, 'inner' => 2]];
"#;
        let issues = run_check(code);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("inner"));
    }

    #[test]
    fn test_three_duplicate_keys() {
        let code = r#"<?php
$a = ['a' => 1, 'b' => 2, 'a' => 3, 'a' => 4];
"#;
        let issues = run_check(code);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("3")); // count is 3
    }
}
