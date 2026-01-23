//! Check for calls to undefined static methods (Level 0)

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::{HashMap, HashSet};

/// Checks for static method calls like Foo::bar()
pub struct CallStaticMethodsCheck;

impl Check for CallStaticMethodsCheck {
    fn id(&self) -> &'static str {
        "staticMethod.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects calls to undefined static methods"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = StaticMethodCallVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_methods: HashMap::new(),
            builtin_classes: ctx.builtin_classes,
            symbol_table: ctx.symbol_table,
            current_namespace: String::new(),
            use_fqn_map: HashMap::new(),
            issues: Vec::new(),
        };

        // First pass: collect class definitions with methods and use statements
        visitor.collect_definitions(program);

        // Second pass: check static method calls
        visitor.visit_program(program, ctx.source);

        visitor.issues
    }
}

struct StaticMethodCallVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    class_methods: HashMap<String, HashSet<String>>, // class name -> method names
    builtin_classes: &'s [&'static str],
    symbol_table: Option<&'s SymbolTable>,
    current_namespace: String,
    use_fqn_map: HashMap<String, String>, // short name -> FQN
    issues: Vec<Issue>,
}

impl<'s> StaticMethodCallVisitor<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_from_stmt(stmt, "");
        }
    }

    fn collect_from_stmt<'a>(&mut self, stmt: &Statement<'a>, namespace: &str) {
        match stmt {
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span);
                let fqn = if namespace.is_empty() {
                    class_name.to_string()
                } else {
                    format!("{}\\{}", namespace, class_name)
                };
                let mut methods = HashSet::new();

                // Collect all methods (both static and non-static, as static:: can call non-static)
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span).to_lowercase();
                        methods.insert(method_name);
                    }
                }

                // Store both short name and FQN
                self.class_methods.insert(class_name.to_lowercase(), methods.clone());
                self.class_methods.insert(fqn.to_lowercase(), methods);
            }
            Statement::Use(use_stmt) => {
                self.collect_use_imports(use_stmt);
            }
            Statement::Namespace(ns) => {
                let ns_name = if let Some(ref name) = ns.name {
                    let span = name.span();
                    self.source[span.start.offset as usize..span.end.offset as usize].to_string()
                } else {
                    String::new()
                };

                if self.current_namespace.is_empty() {
                    self.current_namespace = ns_name.clone();
                }

                match &ns.body {
                    NamespaceBody::Implicit(body) => {
                        for inner in body.statements.iter() {
                            self.collect_from_stmt(inner, &ns_name);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.collect_from_stmt(inner, &ns_name);
                        }
                    }
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_from_stmt(inner, namespace);
                }
            }
            _ => {}
        }
    }

    fn collect_use_imports<'a>(&mut self, use_stmt: &Use<'a>) {
        let use_span = use_stmt.span();
        let use_text = &self.source[use_span.start.offset as usize..use_span.end.offset as usize];

        let text = use_text
            .trim_start_matches("use")
            .trim_start()
            .trim_start_matches("function")
            .trim_start_matches("const")
            .trim()
            .trim_end_matches(';')
            .trim();

        if let Some(brace_start) = text.find('{') {
            let prefix = text[..brace_start].trim().trim_end_matches('\\');
            if let Some(brace_end) = text.find('}') {
                let group_content = &text[brace_start + 1..brace_end];
                for item in group_content.split(',') {
                    let item = item.trim();
                    if let Some(as_pos) = item.to_lowercase().find(" as ") {
                        let class_part = item[..as_pos].trim();
                        let alias = item[as_pos + 4..].trim();
                        let fqn = format!("{}\\{}", prefix, class_part);
                        self.use_fqn_map.insert(alias.to_lowercase(), fqn);
                    } else {
                        let name = item.rsplit('\\').next().unwrap_or(item).trim();
                        if !name.is_empty() {
                            let fqn = format!("{}\\{}", prefix, item.trim());
                            self.use_fqn_map.insert(name.to_lowercase(), fqn);
                        }
                    }
                }
            }
        } else if let Some(as_pos) = text.to_lowercase().find(" as ") {
            let fqn = text[..as_pos].trim().to_string();
            let alias = text[as_pos + 4..].trim();
            self.use_fqn_map.insert(alias.to_lowercase(), fqn);
        } else {
            let name = text.rsplit('\\').next().unwrap_or(text).trim();
            if !name.is_empty() {
                self.use_fqn_map.insert(name.to_lowercase(), text.to_string());
            }
        }
    }

    /// Resolve a class name to FQN using use statements and current namespace
    fn resolve_class_name(&self, name: &str) -> String {
        if name.starts_with('\\') {
            return name[1..].to_string();
        }

        let name_lower = name.to_lowercase();

        // Check use map first
        if let Some(fqn) = self.use_fqn_map.get(&name_lower) {
            return fqn.clone();
        }

        // Prepend current namespace
        if !self.current_namespace.is_empty() {
            format!("{}\\{}", self.current_namespace, name)
        } else {
            name.to_string()
        }
    }

    /// Built-in PHP enum methods (PHP 8.1+)
    const ENUM_BUILTIN_METHODS: &'static [&'static str] = &["cases", "from", "tryfrom"];

    /// Check if a class has a method (local or symbol table)
    /// Returns (found_class, has_method)
    fn class_has_method(&self, class_name: &str, method_name: &str) -> (bool, bool) {
        let class_lower = class_name.to_lowercase();
        let method_lower = method_name.to_lowercase();

        // Check local definitions first
        if let Some(methods) = self.class_methods.get(&class_lower) {
            return (true, methods.contains(&method_lower));
        }

        // Check symbol table
        if let Some(st) = self.symbol_table {
            if let Some(class_info) = st.get_class(&class_lower) {
                // Check direct method
                if class_info.has_method(&method_lower) {
                    return (true, true);
                }

                // For enums, check built-in enum methods
                if class_info.kind == crate::symbols::ClassKind::Enum {
                    if Self::ENUM_BUILTIN_METHODS.contains(&method_lower.as_str()) {
                        return (true, true);
                    }
                }

                // Check parent class methods (inheritance)
                if let Some(ref parent_name) = class_info.parent {
                    if let Some(parent_info) = st.get_class(&parent_name.to_lowercase()) {
                        if parent_info.has_method(&method_lower) {
                            return (true, true);
                        }
                    }
                }

                // Check trait methods
                for trait_name in &class_info.traits {
                    if let Some(trait_info) = st.get_class(&trait_name.to_lowercase()) {
                        if trait_info.has_method(&method_lower) {
                            return (true, true);
                        }
                    }
                }

                // Class found but method not found
                return (true, false);
            }
        }

        // Class not found in either - can't verify
        (false, true)
    }

    /// Check if a class is known (local or symbol table)
    fn is_class_known(&self, class_name: &str) -> bool {
        let class_lower = class_name.to_lowercase();

        // Check local definitions
        if self.class_methods.contains_key(&class_lower) {
            return true;
        }

        // Check symbol table
        if let Some(st) = self.symbol_table {
            if st.get_class(&class_lower).is_some() {
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

impl<'a, 's> Visitor<'a> for StaticMethodCallVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Check for Class::method() calls
        if let Expression::Call(Call::StaticMethod(call)) = expr {
            // Get class name from the target
            let class_name = match &*call.class {
                Expression::Identifier(ident) => {
                    Some(self.get_span_text(&ident.span()).to_string())
                }
                _ => None,
            };

            // Get method name
            let method_info = match &call.method {
                ClassLikeMemberSelector::Identifier(ident) => {
                    Some((self.get_span_text(&ident.span).to_string(), ident.span))
                }
                _ => None,
            };

            if let (Some(class), Some((method, method_span))) = (class_name, method_info) {
                // Skip self, static, parent
                if class.eq_ignore_ascii_case("self")
                    || class.eq_ignore_ascii_case("static")
                    || class.eq_ignore_ascii_case("parent")
                {
                    return true;
                }

                // Normalize class name (strip leading backslash for built-in check)
                let normalized_class = class.trim_start_matches('\\');

                // Skip built-in classes - they have many methods we don't track
                if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(normalized_class)) {
                    return true;
                }

                // Resolve class name to FQN
                let resolved_class = self.resolve_class_name(&class);

                // Check if the class has this method
                let (class_found, has_method) = self.class_has_method(&resolved_class, &method);

                // Only report if class is known and method doesn't exist
                if class_found && !has_method {
                    let (line, col) = self.get_line_col(method_span.start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "staticMethod.notFound",
                            format!(
                                "Call to an undefined static method {}::{}().",
                                resolved_class, method
                            ),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("staticMethod.notFound"),
                    );
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_method_check_level() {
        let check = CallStaticMethodsCheck;
        assert_eq!(check.level(), 0);
    }
}
