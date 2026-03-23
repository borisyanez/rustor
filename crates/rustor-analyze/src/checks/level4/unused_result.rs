//! Unused function result check (Level 4)
//!
//! Detects when a function that returns a value has its result discarded.
//! PHPStan identifier: function.resultUnused

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashSet;
use std::path::PathBuf;

/// Check for unused function results
pub struct UnusedResultCheck;

impl Check for UnusedResultCheck {
    fn id(&self) -> &'static str {
        "function.resultUnused"
    }

    fn description(&self) -> &'static str {
        "Detects when a pure function result is discarded"
    }

    fn level(&self) -> u8 {
        4
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = UnusedResultVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            issues: Vec::new(),
            pure_functions: build_pure_function_set(),
        };

        visitor.analyze_program(program);
        visitor.issues
    }
}

/// Build a set of pure functions whose results should be used
fn build_pure_function_set() -> HashSet<&'static str> {
    let mut set = HashSet::new();

    // String functions
    set.insert("strlen");
    set.insert("substr");
    set.insert("strpos");
    set.insert("strrpos");
    set.insert("str_replace");
    set.insert("str_pad");
    set.insert("trim");
    set.insert("ltrim");
    set.insert("rtrim");
    set.insert("strtolower");
    set.insert("strtoupper");
    set.insert("ucfirst");
    set.insert("lcfirst");
    set.insert("ucwords");
    set.insert("sprintf");
    set.insert("implode");
    set.insert("explode");
    set.insert("join");
    set.insert("chunk_split");
    set.insert("wordwrap");
    set.insert("nl2br");
    set.insert("strip_tags");
    set.insert("htmlspecialchars");
    set.insert("htmlentities");
    set.insert("html_entity_decode");
    set.insert("addslashes");
    set.insert("stripslashes");
    set.insert("quotemeta");
    set.insert("preg_replace");
    // Removed: preg_match and preg_match_all modify $matches by reference
    // Common pattern: preg_match($pattern, $subject, $matches); then use $matches array
    // set.insert("preg_match");
    // set.insert("preg_match_all");
    set.insert("str_split");
    set.insert("number_format");

    // Array functions
    set.insert("count");
    set.insert("sizeof");
    set.insert("array_merge");
    set.insert("array_combine");
    set.insert("array_keys");
    set.insert("array_values");
    set.insert("array_unique");
    set.insert("array_flip");
    set.insert("array_reverse");
    set.insert("array_filter");
    // Removed: array_map is often called for side effects of the callback
    // Common pattern: array_map(fn($x) => $this->process($x), $items); for side effects
    // set.insert("array_map");
    set.insert("array_reduce");
    set.insert("array_column");
    set.insert("array_slice");
    set.insert("array_chunk");
    set.insert("array_search");
    set.insert("array_key_exists");
    set.insert("in_array");
    set.insert("array_diff");
    set.insert("array_intersect");
    set.insert("array_sum");
    set.insert("array_product");
    set.insert("max");
    set.insert("min");
    set.insert("range");

    // Math functions
    set.insert("abs");
    set.insert("ceil");
    set.insert("floor");
    set.insert("round");
    set.insert("sqrt");
    set.insert("pow");
    set.insert("exp");
    set.insert("log");
    set.insert("log10");
    set.insert("sin");
    set.insert("cos");
    set.insert("tan");
    set.insert("rand");
    set.insert("mt_rand");

    // Type checking
    set.insert("gettype");
    set.insert("is_array");
    set.insert("is_bool");
    set.insert("is_callable");
    set.insert("is_float");
    set.insert("is_int");
    set.insert("is_null");
    set.insert("is_numeric");
    set.insert("is_object");
    set.insert("is_string");
    set.insert("is_resource");
    set.insert("isset");
    set.insert("empty");

    // JSON
    set.insert("json_encode");
    // Removed: json_decode is often used for validation without capturing result
    // Common pattern: json_decode($data); if (json_last_error() === JSON_ERROR_NONE) { ... }
    // set.insert("json_decode");

    // Serialization
    set.insert("serialize");
    set.insert("unserialize");

    // Date/time
    set.insert("date");
    set.insert("time");
    set.insert("mktime");
    set.insert("strtotime");
    set.insert("strftime");
    set.insert("gmdate");

    // Misc
    set.insert("compact");
    set.insert("get_class");
    set.insert("get_parent_class");
    set.insert("class_exists");
    set.insert("interface_exists");
    set.insert("function_exists");
    set.insert("method_exists");
    set.insert("property_exists");
    set.insert("defined");
    set.insert("constant");
    set.insert("get_defined_vars");
    set.insert("get_object_vars");
    set.insert("base64_encode");
    set.insert("base64_decode");
    set.insert("md5");
    set.insert("sha1");
    set.insert("hash");
    set.insert("crc32");
    set.insert("ord");
    set.insert("chr");
    set.insert("bin2hex");
    set.insert("hex2bin");
    set.insert("pack");
    set.insert("unpack");

    set
}

struct UnusedResultVisitor<'s> {
    source: &'s str,
    file_path: PathBuf,
    issues: Vec<Issue>,
    pure_functions: HashSet<&'static str>,
}

impl<'s> UnusedResultVisitor<'s> {
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
            Statement::Expression(expr_stmt) => {
                // Check if this is a pure function call whose result is discarded
                self.check_unused_result(&expr_stmt.expression);
            }
            Statement::Function(func) => {
                self.check_block(&func.body);
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            self.check_block(body);
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
                self.check_block(block);
            }
            Statement::If(if_stmt) => {
                self.check_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.check_while_body(&while_stmt.body);
            }
            Statement::For(for_stmt) => {
                self.check_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach_stmt) => {
                self.check_foreach_body(&foreach_stmt.body);
            }
            Statement::Try(try_stmt) => {
                self.check_block(&try_stmt.block);
                for catch in try_stmt.catch_clauses.iter() {
                    self.check_block(&catch.block);
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    self.check_block(&finally.block);
                }
            }
            Statement::Switch(switch) => {
                self.check_switch_body(&switch.body);
            }
            _ => {}
        }
    }

    fn check_block<'a>(&mut self, block: &Block<'a>) {
        for stmt in block.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn check_if_body<'a>(&mut self, body: &IfBody<'a>) {
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
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
                for else_if in block.else_if_clauses.iter() {
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

    fn check_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => self.analyze_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
        }
    }

    fn check_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => self.analyze_statement(stmt),
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
        }
    }

    fn check_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => self.analyze_statement(stmt),
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.analyze_statement(stmt);
                }
            }
        }
    }

    fn check_switch_body<'a>(&mut self, body: &SwitchBody<'a>) {
        let cases = match body {
            SwitchBody::BraceDelimited(b) => &b.cases,
            SwitchBody::ColonDelimited(b) => &b.cases,
        };

        for case in cases.iter() {
            let stmts = match case {
                SwitchCase::Expression(c) => &c.statements,
                SwitchCase::Default(d) => &d.statements,
            };
            for stmt in stmts.iter() {
                self.analyze_statement(stmt);
            }
        }
    }

    fn check_unused_result<'a>(&mut self, expr: &Expression<'a>) {
        if let Expression::Call(Call::Function(func_call)) = expr {
            // Get function name
            if let Some(func_name) = self.extract_function_name(func_call) {
                let func_lower = func_name.to_lowercase();
                if self.pure_functions.contains(func_lower.as_str()) {
                    let (line, col) = self.get_line_col(func_call.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "function.resultUnused",
                            format!(
                                "Call to function {}() on a separate line has no effect.",
                                func_name
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("function.resultUnused"),
                    );
                }
            }
        }
    }

    fn extract_function_name<'a>(&self, call: &FunctionCall<'a>) -> Option<String> {
        match &*call.function {
            Expression::Identifier(id) => {
                Some(self.get_span_text(&id.span()).to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_functions_set() {
        let set = build_pure_function_set();
        assert!(set.contains("strlen"));
        assert!(set.contains("count"));
        assert!(set.contains("array_filter")); // array_filter is pure (returns new array)
        assert!(!set.contains("array_map")); // array_map is not in set (often used for side effects)
        assert!(!set.contains("echo")); // echo is not pure
    }
}
