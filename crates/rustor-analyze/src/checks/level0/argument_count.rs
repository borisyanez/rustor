//! Check for wrong argument counts in function and constructor calls (Level 0)

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::HashMap;

/// Checks for function and constructor calls with wrong number of arguments
pub struct ArgumentCountCheck;

impl Check for ArgumentCountCheck {
    fn id(&self) -> &'static str {
        "arguments.count"
    }

    fn description(&self) -> &'static str {
        "Detects function calls with wrong argument count"
    }

    fn level(&self) -> u8 {
        0 // PHPStan checks this at level 0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = ArgumentCountVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            function_signatures: HashMap::new(),
            class_constructors: HashMap::new(),
            class_names: HashMap::new(), // lowercase -> original
            builtin_classes: ctx.builtin_classes,
            symbol_table: ctx.symbol_table,
            current_namespace: String::new(),
            use_fqn_map: HashMap::new(),
            analysis_level: ctx.analysis_level,
            issues: Vec::new(),
        };

        // First pass: collect function signatures
        visitor.collect_definitions(program);

        // Second pass: check function calls
        visitor.visit_program(program, ctx.source);

        visitor.issues
    }
}

/// Information about function parameters
#[derive(Debug, Clone)]
struct FunctionSignature {
    min_args: usize,
    max_args: Option<usize>, // None means variadic
}

struct ArgumentCountVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    function_signatures: HashMap<String, FunctionSignature>,
    class_constructors: HashMap<String, FunctionSignature>, // class name (lowercase) -> constructor signature
    class_names: HashMap<String, String>,                    // class name (lowercase) -> original name
    builtin_classes: &'s [&'static str],
    symbol_table: Option<&'s SymbolTable>,
    current_namespace: String,
    use_fqn_map: HashMap<String, String>, // short name -> FQN
    analysis_level: u8, // Analysis level - "too many args" only reported at level 2+
    issues: Vec<Issue>,
}

impl<'s> ArgumentCountVisitor<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_from_stmt(stmt);
        }
    }

    fn collect_from_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                let name = self.get_span_text(&func.name.span).to_lowercase();
                let sig = self.analyze_parameters(&func.parameter_list);
                self.function_signatures.insert(name, sig);
            }
            Statement::Class(class) => {
                let original_name = self.get_span_text(&class.name.span).to_string();
                let class_lower = original_name.to_lowercase();

                // Find the __construct method
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span).to_lowercase();
                        if method_name == "__construct" {
                            let sig = self.analyze_parameters(&method.parameter_list);
                            self.class_constructors.insert(class_lower.clone(), sig);
                            break;
                        }
                    }
                }

                self.class_names.insert(class_lower, original_name);
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
                            self.collect_from_stmt(inner);
                        }
                    }
                    NamespaceBody::BraceDelimited(body) => {
                        for inner in body.statements.iter() {
                            self.collect_from_stmt(inner);
                        }
                    }
                }
            }
            Statement::Use(use_stmt) => {
                self.collect_use_imports(use_stmt);
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_from_stmt(inner);
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

    /// Resolve a short class name to its FQN using use-statements and current namespace
    fn resolve_class_name(&self, name: &str) -> String {
        if name.starts_with('\\') {
            return name[1..].to_string();
        }
        let name_lower = name.to_lowercase();
        if let Some(fqn) = self.use_fqn_map.get(&name_lower) {
            return fqn.clone();
        }
        if !self.current_namespace.is_empty() {
            format!("{}\\{}", self.current_namespace, name)
        } else {
            name.to_string()
        }
    }

    fn check_arg_count_from_sig(
        &mut self,
        arg_count: usize,
        required: usize,
        max: Option<usize>,
        message_too_few: String,
        message_too_many: String,
        line: usize,
        col: usize,
    ) {
        if arg_count < required {
            self.issues.push(
                Issue::error("arguments.count", message_too_few, self.file_path.clone(), line, col)
                    .with_identifier("arguments.count"),
            );
        } else if let Some(max_count) = max {
            if arg_count > max_count && self.analysis_level >= 1 {
                self.issues.push(
                    Issue::error("arguments.count", message_too_many, self.file_path.clone(), line, col)
                        .with_identifier("arguments.count"),
                );
            }
        }
    }

    fn analyze_parameters(&self, params: &FunctionLikeParameterList<'_>) -> FunctionSignature {
        let mut min_args = 0;
        let mut has_variadic = false;

        for param in params.parameters.iter() {
            if param.ellipsis.is_some() {
                has_variadic = true;
            } else if param.default_value.is_none() {
                min_args += 1;
            }
        }

        let max_args = if has_variadic {
            None
        } else {
            Some(params.parameters.len())
        };

        FunctionSignature { min_args, max_args }
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

impl<'a, 's> Visitor<'a> for ArgumentCountVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        match expr {
            Expression::Call(Call::Function(call)) => {
                // Get function name
                let name_span = call.function.span();
                let name = self.get_span_text(&name_span);

                // Skip dynamic calls
                if name.starts_with('$') {
                    return true;
                }

                let name_lower = name.to_lowercase();
                let arg_count = call.argument_list.arguments.len();

                // Check if we have a local signature for this function
                if let Some(sig) = self.function_signatures.get(&name_lower).cloned() {
                    let (line, col) = self.get_line_col(name_span.start.offset as usize);
                    let too_few = format!(
                        "Function {} invoked with {} parameter{}, {} required.",
                        name, arg_count, if arg_count == 1 { "" } else { "s" }, sig.min_args
                    );
                    let too_many = if let Some(max) = sig.max_args {
                        if sig.min_args == max {
                            format!(
                                "Function {} invoked with {} parameter{}, {} required.",
                                name, arg_count, if arg_count == 1 { "" } else { "s" }, max
                            )
                        } else {
                            format!(
                                "Function {} invoked with {} parameter{}, {}-{} required.",
                                name, arg_count, if arg_count == 1 { "" } else { "s" }, sig.min_args, max
                            )
                        }
                    } else {
                        String::new()
                    };
                    self.check_arg_count_from_sig(arg_count, sig.min_args, sig.max_args, too_few, too_many, line, col);
                } else if let Some(symbol_table) = self.symbol_table {
                    // Try symbol table for cross-file functions
                    // Try plain name first, then with current namespace prefix
                    let fqn = self.resolve_class_name(name); // resolve uses use_fqn_map / namespace
                    let func_info = symbol_table
                        .get_function(&fqn)
                        .or_else(|| symbol_table.get_function(&name_lower));
                    if let Some(func_info) = func_info {
                        let required = func_info.required_args();
                        let max = func_info.max_args();
                        // Only report if parameters are actually defined (non-empty)
                        // to avoid false positives on built-in stubs with no param info
                        if !func_info.parameters.is_empty() {
                            let (line, col) = self.get_line_col(name_span.start.offset as usize);
                            let func_display = func_info.name.clone();
                            let too_few = format!(
                                "Function {} invoked with {} parameter{}, {} required.",
                                func_display, arg_count, if arg_count == 1 { "" } else { "s" }, required
                            );
                            let too_many = if let Some(max_count) = max {
                                format!(
                                    "Function {} invoked with {} parameter{}, {}-{} required.",
                                    func_display, arg_count, if arg_count == 1 { "" } else { "s" }, required, max_count
                                )
                            } else {
                                String::new()
                            };
                            self.check_arg_count_from_sig(arg_count, required, max, too_few, too_many, line, col);
                        }
                    }
                }
            }
            Expression::Call(Call::StaticMethod(call)) => {
                // Get class name and method name
                let class_name = match &*call.class {
                    Expression::Identifier(ident) => {
                        Some(self.get_span_text(&ident.span()).to_string())
                    }
                    _ => None,
                };
                let method_name = match &call.method {
                    ClassLikeMemberSelector::Identifier(ident) => {
                        Some(self.get_span_text(&ident.span).to_string())
                    }
                    _ => None,
                };

                if let (Some(class_name), Some(method_name)) = (class_name, method_name) {
                    // Skip dynamic/special calls
                    if class_name == "self" || class_name == "static" || class_name == "parent" {
                        return true;
                    }

                    let arg_count = call.argument_list.arguments.len();
                    let call_span = call.class.span();

                    if let Some(symbol_table) = self.symbol_table {
                        let fqn = self.resolve_class_name(&class_name);
                        let class_info = symbol_table
                            .get_class(&fqn)
                            .or_else(|| symbol_table.get_class(&class_name));
                        if let Some(class_info) = class_info {
                            if let Some(method_info) = class_info.get_method(&method_name) {
                                // Skip if method has no parameter info (cache stub)
                                if !(method_info.parameters.is_empty() && arg_count > 0) {
                                let required = method_info.required_args();
                                let max = method_info.max_args();
                                let display_class = &class_info.full_name;
                                let display_method = &method_info.name;
                                let (line, col) = self.get_line_col(call_span.start.offset as usize);
                                let too_few = format!(
                                    "Static method {}::{}() invoked with {} parameter{}, {} required.",
                                    display_class, display_method,
                                    arg_count, if arg_count == 1 { "" } else { "s" }, required
                                );
                                let too_many = if let Some(max_count) = max {
                                    format!(
                                        "Static method {}::{}() invoked with {} parameter{}, {}-{} required.",
                                        display_class, display_method,
                                        arg_count, if arg_count == 1 { "" } else { "s" }, required, max_count
                                    )
                                } else {
                                    String::new()
                                };
                                self.check_arg_count_from_sig(arg_count, required, max, too_few, too_many, line, col);
                                }
                            }
                        }
                    }
                }
            }
            Expression::Instantiation(inst) => {
                // Get class name
                let class_name = match &*inst.class {
                    Expression::Identifier(ident) => {
                        Some(self.get_span_text(&ident.span()).to_string())
                    }
                    _ => None,
                };

                if let Some(name) = class_name {
                    // Skip built-in classes
                    if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&name)) {
                        return true;
                    }

                    let name_lower = name.to_lowercase();
                    let arg_count = inst
                        .argument_list
                        .as_ref()
                        .map(|al| al.arguments.len())
                        .unwrap_or(0);

                    // Check if we have a constructor signature for this class
                    if let Some(sig) = self.class_constructors.get(&name_lower).cloned() {
                        let class_span = inst.class.span();
                        let display_name = self
                            .class_names
                            .get(&name_lower)
                            .cloned()
                            .unwrap_or(name.clone());

                        if arg_count < sig.min_args {
                            let (line, col) = self.get_line_col(class_span.start.offset as usize);
                            self.issues.push(
                                Issue::error(
                                    "arguments.count",
                                    format!(
                                        "Class {} constructor invoked with {} parameter{}, {} required.",
                                        display_name,
                                        arg_count,
                                        if arg_count == 1 { "" } else { "s" },
                                        sig.min_args
                                    ),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("arguments.count"),
                            );
                        } else if let Some(max) = sig.max_args {
                            // PHPStan reports "too many arguments" at level 1+
                            if arg_count > max && self.analysis_level >= 1 {
                                let (line, col) = self.get_line_col(class_span.start.offset as usize);
                                self.issues.push(
                                    Issue::error(
                                        "arguments.count",
                                        format!(
                                            "Class {} constructor invoked with {} parameter{}, {} required.",
                                            display_name,
                                            arg_count,
                                            if arg_count == 1 { "" } else { "s" },
                                            max
                                        ),
                                        self.file_path.clone(),
                                        line,
                                        col,
                                    )
                                    .with_identifier("arguments.count"),
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_count_check_level() {
        let check = ArgumentCountCheck;
        assert_eq!(check.level(), 0); // Should be level 0 like PHPStan
    }
}
