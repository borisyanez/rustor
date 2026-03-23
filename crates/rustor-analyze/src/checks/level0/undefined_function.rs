//! Check for calls to undefined functions

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::HashSet;

pub struct UndefinedFunctionCheck;

impl Check for UndefinedFunctionCheck {
    fn id(&self) -> &'static str {
        "function.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects calls to undefined functions"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = UndefinedFunctionVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            builtin_functions: ctx.builtin_functions,
            symbol_table: ctx.symbol_table,
            defined_functions: HashSet::new(),
            imported_functions: HashSet::new(),
            issues: Vec::new(),
        };

        // First pass: collect function definitions and imports
        visitor.collect_definitions(program);

        // Second pass: check function calls
        visitor.visit_program(program, ctx.source);

        visitor.issues
    }
}

struct UndefinedFunctionVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    builtin_functions: &'s [&'static str],
    symbol_table: Option<&'s SymbolTable>,
    defined_functions: HashSet<String>,
    imported_functions: HashSet<String>,
    issues: Vec<Issue>,
}

impl<'s> UndefinedFunctionVisitor<'s> {
    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_definitions_in_stmt(stmt);
        }
    }

    fn collect_definitions_in_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                let name = &self.source[func.name.span.start.offset as usize..func.name.span.end.offset as usize];
                self.defined_functions.insert(name.to_lowercase());
            }
            // Collect use function imports
            Statement::Use(use_stmt) => {
                let use_span = use_stmt.span();
                let use_text = self.source[use_span.start.offset as usize..use_span.end.offset as usize].to_string();
                self.collect_function_imports(&use_text);
            }
            Statement::Namespace(ns) => {
                match &ns.body {
                    NamespaceBody::Implicit(body) => {
                        for inner in body.statements.iter() {
                            self.collect_definitions_in_stmt(inner);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.collect_definitions_in_stmt(inner);
                        }
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_definitions_in_stmt(inner);
                }
            }
            _ => {}
        }
    }

    /// Collect function imports from "use function" statements
    fn collect_function_imports(&mut self, use_text: &str) {
        // Only process "use function" statements
        let trimmed = use_text.trim_start_matches("use").trim_start();
        if !trimmed.starts_with("function") {
            return;
        }

        // Remove 'use function' and semicolon
        let text = trimmed
            .trim_start_matches("function")
            .trim()
            .trim_end_matches(';')
            .trim();

        // Handle grouped imports: Namespace\{func1, func2}
        if let Some(brace_start) = text.find('{') {
            let prefix = text[..brace_start].trim().trim_end_matches('\\');
            if let Some(brace_end) = text.find('}') {
                let group_content = &text[brace_start + 1..brace_end];
                for item in group_content.split(',') {
                    let item = item.trim();
                    // Handle "func as alias"
                    if let Some(as_pos) = item.to_lowercase().find(" as ") {
                        let alias = item[as_pos + 4..].trim();
                        self.imported_functions.insert(alias.to_lowercase());
                    } else {
                        // Just "func" - use the function name itself
                        let name = item.rsplit('\\').next().unwrap_or(item).trim();
                        if !name.is_empty() {
                            self.imported_functions.insert(name.to_lowercase());
                        }
                    }
                }
            }
        } else {
            // Simple import: Namespace\func or Namespace\func as alias
            if let Some(as_pos) = text.to_lowercase().find(" as ") {
                let alias = text[as_pos + 4..].trim();
                self.imported_functions.insert(alias.to_lowercase());
            } else {
                // Get last segment (the function name)
                let name = text.rsplit('\\').next().unwrap_or(text).trim();
                if !name.is_empty() {
                    self.imported_functions.insert(name.to_lowercase());
                }
            }
        }
    }

    fn is_defined(&self, name: &str) -> bool {
        let lower_name = name.to_lowercase();

        // Check builtin functions (case-insensitive)
        if self.builtin_functions.iter().any(|f| f.eq_ignore_ascii_case(name)) {
            return true;
        }

        // Check user-defined functions in current file
        if self.defined_functions.contains(&lower_name) {
            return true;
        }

        // Check if imported via "use function"
        if self.imported_functions.contains(&lower_name) {
            return true;
        }

        // Check symbol table from autoload scanning
        if let Some(st) = self.symbol_table {
            if st.get_function(name).is_some() {
                return true;
            }
        }

        false
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
}

impl<'a, 's> Visitor<'a> for UndefinedFunctionVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        if let Expression::Call(Call::Function(call)) = expr {
            // Get the function name
            let func_span = call.function.span();
            let name = &self.source[func_span.start.offset as usize..func_span.end.offset as usize];

            // Skip fully qualified names (they may reference imported functions)
            // and names with namespace separator (we can't resolve those without autoloader)
            if name.contains('\\') {
                return true;
            }

            // Skip dynamic calls like $func()
            if name.starts_with('$') {
                return true;
            }

            // Skip common special functions that aren't in our builtin list
            // These are usually framework/library functions
            if is_likely_framework_function(name) {
                return true;
            }

            if !self.is_defined(name) {
                let (line, col) = self.get_line_col(func_span.start.offset as usize);
                self.issues.push(
                    Issue::error(
                        "function.notFound",
                        format!("Function {} not found.", name),
                        self.file_path.clone(),
                        line,
                        col,
                    )
                    .with_identifier("function.notFound"),
                );
            }
        }
        true
    }
}

/// Check if a function name looks like a framework function
/// These are commonly defined by autoloaded code
fn is_likely_framework_function(name: &str) -> bool {
    // Common Laravel helpers
    let laravel_helpers = [
        "app", "config", "env", "route", "url", "view", "redirect", "response",
        "request", "session", "trans", "lang", "__", "old", "csrf_field", "csrf_token",
        "method_field", "abort", "auth", "back", "bcrypt", "cache", "collect",
        "cookie", "dispatch", "event", "factory", "info", "logger", "now",
        "policy", "public_path", "storage_path", "resource_path", "base_path",
        "database_path", "app_path", "config_path", "report", "rescue", "resolve",
        "validator", "with", "dd", "dump", "data_get", "data_set", "head", "last",
        "value", "tap", "retry", "throw_if", "throw_unless", "optional",
    ];

    // Common Symfony helpers
    let symfony_helpers = ["dump", "dd"];

    // Common WordPress helpers
    let wordpress_helpers = [
        "add_action", "add_filter", "apply_filters", "do_action", "get_option",
        "update_option", "delete_option", "wp_enqueue_script", "wp_enqueue_style",
        "get_post", "get_posts", "the_content", "the_title", "the_permalink",
        "esc_html", "esc_attr", "esc_url", "wp_nonce_field", "check_admin_referer",
        "__", "_e", "_x", "_n", "sprintf", "get_template_part",
    ];

    laravel_helpers.iter().any(|h| h.eq_ignore_ascii_case(name))
        || symfony_helpers.iter().any(|h| h.eq_ignore_ascii_case(name))
        || wordpress_helpers.iter().any(|h| h.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_framework_function() {
        assert!(is_likely_framework_function("app"));
        assert!(is_likely_framework_function("config"));
        assert!(is_likely_framework_function("dd"));
        assert!(!is_likely_framework_function("my_custom_func"));
    }
}
