//! Check for calls to undefined methods on known types (Level 2)
//! Also validates method argument counts.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Visitor;
use std::collections::HashMap;

/// Method signature information
#[derive(Debug, Clone)]
struct MethodInfo {
    /// Original method name (preserving case)
    name: String,
    /// Minimum required parameters
    min_params: usize,
    /// Maximum parameters (None if variadic)
    max_params: Option<usize>,
}

/// Checks for method calls on objects where we know the type
pub struct CallMethodsCheck;

impl Check for CallMethodsCheck {
    fn id(&self) -> &'static str {
        "method.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects method calls on known types where method doesn't exist"
    }

    fn level(&self) -> u8 {
        2
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = MethodCallVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            class_methods: HashMap::new(),
            class_names: HashMap::new(),
            variable_types: HashMap::new(),
            param_type_stack: Vec::new(),
            builtin_classes: ctx.builtin_classes,
            symbol_table: ctx.symbol_table,
            issues: Vec::new(),
            check_arg_counts: true,
            current_namespace: String::new(),
            use_fqn_map: HashMap::new(),
        };

        // First pass: collect class methods
        visitor.collect_definitions(program);

        // Second pass: check method calls (with function scope tracking)
        visitor.analyze_program(program);

        visitor.issues
    }
}

struct MethodCallVisitor<'s> {
    source: &'s str,
    file_path: std::path::PathBuf,
    class_methods: HashMap<String, HashMap<String, MethodInfo>>, // class name (lowercase) -> method name (lowercase) -> info
    class_names: HashMap<String, String>,                         // class name (lowercase) -> original name
    variable_types: HashMap<String, String>,                      // variable name -> class name (original)
    /// Stack of parameter types for nested function scopes
    param_type_stack: Vec<HashMap<String, String>>,
    builtin_classes: &'s [&'static str],
    symbol_table: Option<&'s SymbolTable>,
    issues: Vec<Issue>,
    check_arg_counts: bool,
    /// Current namespace for FQN resolution
    current_namespace: String,
    /// Map of imported class short names to FQN (short name lowercase -> FQN)
    use_fqn_map: HashMap<String, String>,
}

impl<'s> MethodCallVisitor<'s> {
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
            Statement::Class(class) => {
                let original_name = self.get_span_text(&class.name.span).to_string();
                let class_lower = original_name.to_lowercase();
                let mut methods = HashMap::new();

                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span).to_string();
                        let method_lower = method_name.to_lowercase();

                        // Count parameters
                        let params = &method.parameter_list.parameters;
                        let mut min_params = 0;
                        let mut is_variadic = false;

                        for param in params.iter() {
                            if param.ellipsis.is_some() {
                                is_variadic = true;
                            } else if param.default_value.is_none() {
                                min_params += 1;
                            }
                        }

                        let max_params = if is_variadic {
                            None
                        } else {
                            Some(params.len())
                        };

                        methods.insert(
                            method_lower,
                            MethodInfo {
                                name: method_name,
                                min_params,
                                max_params,
                            },
                        );
                    }
                }

                self.class_names.insert(class_lower.clone(), original_name);
                self.class_methods.insert(class_lower, methods);
            }
            Statement::Namespace(ns) => {
                // Extract namespace name
                if let Some(ref name) = ns.name {
                    let span = name.span();
                    self.current_namespace = self.source[span.start.offset as usize..span.end.offset as usize].to_string();
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
                // Extract imports from use statement
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

    /// Collect use imports from a use statement
    fn collect_use_imports<'a>(&mut self, use_stmt: &Use<'a>) {
        let use_span = use_stmt.span();
        let use_text = &self.source[use_span.start.offset as usize..use_span.end.offset as usize];
        self.extract_imports_from_use_text(use_text);
    }

    /// Parse use statement text and extract class aliases
    fn extract_imports_from_use_text(&mut self, use_text: &str) {
        // Remove 'use', 'function', 'const' keywords and semicolon
        let text = use_text
            .trim_start_matches("use")
            .trim_start()
            .trim_start_matches("function")
            .trim_start_matches("const")
            .trim()
            .trim_end_matches(';')
            .trim();

        // Handle grouped imports: Foo\{Bar, Baz as Qux}
        if let Some(brace_start) = text.find('{') {
            let prefix = text[..brace_start].trim().trim_end_matches('\\');
            if let Some(brace_end) = text.find('}') {
                let group_content = &text[brace_start + 1..brace_end];
                for item in group_content.split(',') {
                    let item = item.trim();
                    // Handle "Bar as Baz" - use alias
                    if let Some(as_pos) = item.to_lowercase().find(" as ") {
                        let class_part = item[..as_pos].trim();
                        let alias = item[as_pos + 4..].trim();
                        let fqn = format!("{}\\{}", prefix, class_part);
                        self.use_fqn_map.insert(alias.to_lowercase(), fqn);
                    } else {
                        // Just "Bar" - use last segment
                        let name = item.rsplit('\\').next().unwrap_or(item).trim();
                        if !name.is_empty() {
                            let fqn = format!("{}\\{}", prefix, item.trim());
                            self.use_fqn_map.insert(name.to_lowercase(), fqn);
                        }
                    }
                }
            }
        } else {
            // Simple import: Foo\Bar or Foo\Bar as Baz
            if let Some(as_pos) = text.to_lowercase().find(" as ") {
                let fqn = text[..as_pos].trim().to_string();
                let alias = text[as_pos + 4..].trim();
                self.use_fqn_map.insert(alias.to_lowercase(), fqn);
            } else {
                // Get last segment of namespace
                let name = text.rsplit('\\').next().unwrap_or(text).trim();
                if !name.is_empty() {
                    self.use_fqn_map.insert(name.to_lowercase(), text.to_string());
                }
            }
        }
    }

    /// Resolve a class name to its fully qualified name
    fn resolve_class_name(&self, name: &str) -> String {
        // Already fully qualified
        if name.starts_with('\\') {
            return name[1..].to_string();
        }

        // Contains namespace separator - could be partially qualified
        if name.contains('\\') {
            // Check if first part is an alias
            let first_part = name.split('\\').next().unwrap_or(name);
            if let Some(fqn) = self.use_fqn_map.get(&first_part.to_lowercase()) {
                let rest = &name[first_part.len()..];
                return format!("{}{}", fqn, rest);
            }
            // Try with current namespace
            if !self.current_namespace.is_empty() {
                return format!("{}\\{}", self.current_namespace, name);
            }
            return name.to_string();
        }

        // Check use imports (case-insensitive)
        let lower_name = name.to_lowercase();
        if let Some(fqn) = self.use_fqn_map.get(&lower_name) {
            return fqn.clone();
        }

        // Prepend current namespace if not found in imports
        if !self.current_namespace.is_empty() {
            format!("{}\\{}", self.current_namespace, name)
        } else {
            name.to_string()
        }
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

    /// Extract class name from an instantiation expression
    fn get_instantiation_class<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Instantiation(inst) => match &*inst.class {
                Expression::Identifier(ident) => {
                    Some(self.get_span_text(&ident.span()).to_string())
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Get variable type from parameter stack or variable_types map
    fn get_variable_type(&self, var_name: &str) -> Option<String> {
        // First check parameter stack (innermost scope first)
        for scope in self.param_type_stack.iter().rev() {
            if let Some(type_name) = scope.get(var_name) {
                return Some(type_name.clone());
            }
        }
        // Then check variable assignments
        self.variable_types.get(var_name).cloned()
    }

    /// Check if a method exists on a class (checks both local file and symbol table)
    /// Returns true if the method exists, false otherwise
    fn method_exists(&self, class_name: &str, method_name: &str) -> bool {
        let class_lower = class_name.to_lowercase();
        let method_lower = method_name.to_lowercase();

        // First check local file definitions (by short name)
        if let Some(methods) = self.class_methods.get(&class_lower) {
            if methods.contains_key(&method_lower) {
                return true;
            }
            // Check for __call magic method in local file
            if methods.contains_key("__call") {
                return true;
            }
        }

        // Then check symbol table for cross-file resolution
        if let Some(symbol_table) = self.symbol_table {
            // Resolve to fully qualified name
            let fqn = self.resolve_class_name(class_name);

            // Check direct class by FQN
            if symbol_table.class_has_method(&fqn, method_name) {
                return true;
            }

            // Also try with short name (for classes in global namespace)
            if symbol_table.class_has_method(class_name, method_name) {
                return true;
            }

            // Check for __call magic method (handles dynamic method dispatch)
            if self.has_magic_call_in_hierarchy(&fqn, symbol_table) {
                return true;
            }

            // Check parent classes and traits recursively
            if let Some(class_info) = symbol_table.get_class(&fqn).or_else(|| symbol_table.get_class(class_name)) {
                // Check parent class
                if let Some(parent) = &class_info.parent {
                    if self.method_exists_in_hierarchy(parent, method_name, symbol_table) {
                        return true;
                    }
                }

                // Check traits
                for trait_name in &class_info.traits {
                    if self.method_exists_in_hierarchy(trait_name, method_name, symbol_table) {
                        return true;
                    }
                }

                // Check interfaces (they can have default methods in PHP 8.0+)
                for interface in &class_info.interfaces {
                    if self.method_exists_in_hierarchy(interface, method_name, symbol_table) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if a class or its parents/traits have __call magic method
    fn has_magic_call_in_hierarchy(&self, class_name: &str, symbol_table: &SymbolTable) -> bool {
        // Check this class directly
        if symbol_table.class_has_method(class_name, "__call") {
            return true;
        }

        // Check parent hierarchy
        if let Some(class_info) = symbol_table.get_class(class_name) {
            // Check parent class
            if let Some(parent) = &class_info.parent {
                if self.has_magic_call_in_hierarchy(parent, symbol_table) {
                    return true;
                }
            }

            // Check traits (traits can define __call)
            for trait_name in &class_info.traits {
                if self.has_magic_call_in_hierarchy(trait_name, symbol_table) {
                    return true;
                }
            }
        }

        false
    }

    /// Recursively check if a method exists in a class hierarchy
    fn method_exists_in_hierarchy(&self, class_name: &str, method_name: &str, symbol_table: &SymbolTable) -> bool {
        // Check this class
        if symbol_table.class_has_method(class_name, method_name) {
            return true;
        }

        // Recursively check parent and traits
        if let Some(class_info) = symbol_table.get_class(class_name) {
            if let Some(parent) = &class_info.parent {
                if self.method_exists_in_hierarchy(parent, method_name, symbol_table) {
                    return true;
                }
            }

            for trait_name in &class_info.traits {
                if self.method_exists_in_hierarchy(trait_name, method_name, symbol_table) {
                    return true;
                }
            }
        }

        false
    }

    /// Extract typed parameters from a parameter list
    fn extract_typed_params(&self, params: &FunctionLikeParameterList<'_>) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for param in params.parameters.iter() {
            if let Some(hint) = &param.hint {
                // Get the type name from the hint
                if let Some(type_name) = self.extract_type_name(hint) {
                    let var_name = self.get_span_text(&param.variable.span).to_string();
                    result.insert(var_name, type_name);
                }
            }
        }
        result
    }

    /// Extract type name from a Hint
    fn extract_type_name(&self, hint: &Hint<'_>) -> Option<String> {
        match hint {
            Hint::Identifier(ident) => Some(self.get_span_text(&ident.span()).to_string()),
            Hint::Nullable(nullable) => self.extract_type_name(&nullable.hint),
            Hint::Parenthesized(p) => self.extract_type_name(&p.hint),
            _ => None, // Skip union types, intersection types, etc. for now
        }
    }

    /// Analyze program with scope tracking for typed parameters
    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                // Push parameter types onto stack
                let params = self.extract_typed_params(&func.parameter_list);
                self.param_type_stack.push(params);

                // Analyze function body
                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
                }

                // Pop parameter types
                self.param_type_stack.pop();
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            // Push parameter types onto stack
                            let params = self.extract_typed_params(&method.parameter_list);
                            self.param_type_stack.push(params);

                            // Analyze method body
                            for inner in body.statements.iter() {
                                self.analyze_statement(inner);
                            }

                            // Pop parameter types
                            self.param_type_stack.pop();
                        }
                    }
                }
            }
            Statement::Expression(expr_stmt) => {
                // Track variable assignments
                if let Expression::Assignment(assign) = &expr_stmt.expression {
                    if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                        let var_name = self.get_span_text(&var.span).to_string();
                        if let Some(class_name) = self.get_instantiation_class(assign.rhs) {
                            self.variable_types.insert(var_name, class_name);
                        }
                    }
                }
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
                self.analyze_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.analyze_expression(&foreach.expression);
                self.analyze_foreach_body(&foreach.body);
            }
            Statement::Try(try_stmt) => {
                for inner in try_stmt.block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for catch in try_stmt.catch_clauses.iter() {
                    for inner in catch.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                if let Some(finally) = &try_stmt.finally_clause {
                    for inner in finally.block.statements.iter() {
                        self.analyze_statement(inner);
                    }
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
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.analyze_expression(&else_if.condition);
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

    fn analyze_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => self.analyze_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => self.analyze_statement(stmt),
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => self.analyze_statement(stmt),
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_expression<'a>(&mut self, expr: &Expression<'a>) {
        // Check for $obj->method() calls
        if let Expression::Call(Call::Method(call)) = expr {
            self.check_method_call(call);
        }

        // Recurse into nested expressions
        match expr {
            Expression::Call(call) => match call {
                Call::Method(m) => {
                    self.analyze_expression(&m.object);
                    for arg in m.argument_list.arguments.iter() {
                        self.analyze_argument_value(arg);
                    }
                }
                Call::Function(f) => {
                    for arg in f.argument_list.arguments.iter() {
                        self.analyze_argument_value(arg);
                    }
                }
                Call::StaticMethod(s) => {
                    for arg in s.argument_list.arguments.iter() {
                        self.analyze_argument_value(arg);
                    }
                }
                Call::NullSafeMethod(n) => {
                    self.analyze_expression(&n.object);
                    for arg in n.argument_list.arguments.iter() {
                        self.analyze_argument_value(arg);
                    }
                }
            },
            Expression::Access(access) => match access {
                Access::Property(p) => self.analyze_expression(&p.object),
                Access::NullSafeProperty(p) => self.analyze_expression(&p.object),
                Access::StaticProperty(_) => {}
                Access::ClassConstant(_) => {}
            },
            Expression::Binary(binary) => {
                self.analyze_expression(&binary.lhs);
                self.analyze_expression(&binary.rhs);
            }
            Expression::UnaryPrefix(p) => self.analyze_expression(&p.operand),
            Expression::UnaryPostfix(p) => self.analyze_expression(&p.operand),
            Expression::Parenthesized(p) => self.analyze_expression(&p.expression),
            Expression::Conditional(t) => {
                self.analyze_expression(&t.condition);
                if let Some(then_expr) = &t.then {
                    self.analyze_expression(then_expr);
                }
                self.analyze_expression(&t.r#else);
            }
            Expression::Assignment(a) => {
                self.analyze_expression(&a.rhs);
            }
            Expression::Array(arr) => {
                for elem in arr.elements.iter() {
                    if let ArrayElement::KeyValue(kv) = elem {
                        self.analyze_expression(&kv.value);
                    } else if let ArrayElement::Value(v) = elem {
                        self.analyze_expression(&v.value);
                    }
                }
            }
            _ => {}
        }
    }

    fn analyze_argument_value<'a>(&mut self, arg: &Argument<'a>) {
        match arg {
            Argument::Positional(p) => self.analyze_expression(&p.value),
            Argument::Named(n) => self.analyze_expression(&n.value),
        }
    }

    fn check_method_call<'a>(&mut self, call: &MethodCall<'a>) {
        // Get method name
        let method_info = match &call.method {
            ClassLikeMemberSelector::Identifier(ident) => {
                Some((self.get_span_text(&ident.span).to_string(), ident.span))
            }
            _ => None,
        };

        if let Some((method, method_span)) = method_info {
            let method_lower = method.to_lowercase();
            let arg_count = call.argument_list.arguments.len();

            // Case 1: (new ClassName())->method()
            if let Some(class_name) = self.get_instantiation_class(&call.object) {
                self.check_method_on_class(&class_name, &method, &method_lower, arg_count, method_span);
            }
            // Case 2: $obj->method() where $obj has a known type
            else if let Expression::Variable(Variable::Direct(var)) = &*call.object {
                let var_name = self.get_span_text(&var.span).to_string();
                if let Some(class_name) = self.get_variable_type(&var_name) {
                    self.check_method_on_class(&class_name, &method, &method_lower, arg_count, method_span);
                }
            }
        }
    }

    fn check_method_on_class(
        &mut self,
        class_name: &str,
        method: &str,
        method_lower: &str,
        arg_count: usize,
        method_span: mago_span::Span,
    ) {
        // Skip built-in classes
        if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(class_name)) {
            return;
        }

        let class_lower = class_name.to_lowercase();

        // Check if method exists (local or cross-file)
        if !self.method_exists(class_name, method) {
            // Method not found anywhere
            let (line, col) = self.get_line_col(method_span.start.offset as usize);
            self.issues.push(
                Issue::error(
                    "method.notFound",
                    format!(
                        "Call to an undefined method {}::{}().",
                        class_name, method
                    ),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("method.notFound"),
            );
        } else if let Some(methods) = self.class_methods.get(&class_lower) {
            // Method exists in local file - check argument count
            if let Some(method_info) = methods.get(method_lower) {
                let method_info = method_info.clone();
                self.check_method_args(
                    class_name,
                    &method_info,
                    arg_count,
                    method_span.start.offset as usize,
                );
            }
        }
    }
}

// Keep the old Visitor impl for backwards compatibility but it's not used anymore
impl<'a, 's> Visitor<'a> for MethodCallVisitor<'s> {
    fn visit_statement(&mut self, stmt: &Statement<'a>, _source: &str) -> bool {
        // Track variable assignments: $obj = new ClassName()
        if let Statement::Expression(expr_stmt) = stmt {
            if let Expression::Assignment(assign) = &expr_stmt.expression {
                // Check if left is a variable and right is an instantiation
                if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                    let var_name = self.get_span_text(&var.span).to_string();
                    if let Some(class_name) = self.get_instantiation_class(assign.rhs) {
                        // Store original class name (type tracking preserves case)
                        self.variable_types.insert(var_name, class_name);
                    }
                }
            }
        }
        true
    }

    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Check for $obj->method() calls
        if let Expression::Call(Call::Method(call)) = expr {
            // Get method name
            let method_info = match &call.method {
                ClassLikeMemberSelector::Identifier(ident) => {
                    Some((self.get_span_text(&ident.span).to_string(), ident.span))
                }
                _ => None,
            };

            if let Some((method, method_span)) = method_info {
                let method_lower = method.to_lowercase();
                let arg_count = call.argument_list.arguments.len();

                // Case 1: (new ClassName())->method()
                if let Some(class_name) = self.get_instantiation_class(&call.object) {
                    // Skip built-in classes
                    if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&class_name)) {
                        return true;
                    }

                    let class_lower = class_name.to_lowercase();

                    // Check if method exists (local or cross-file)
                    if !self.method_exists(&class_name, &method) {
                        // Method not found anywhere
                        let (line, col) = self.get_line_col(method_span.start.offset as usize);
                        self.issues.push(
                            Issue::error(
                                "method.notFound",
                                format!(
                                    "Call to an undefined method {}::{}().",
                                    class_name, method
                                ),
                                self.file_path.clone(),
                                line,
                                col,
                            )
                            .with_identifier("method.notFound"),
                        );
                    } else if let Some(methods) = self.class_methods.get(&class_lower) {
                        // Method exists in local file - check argument count
                        if let Some(method_info) = methods.get(&method_lower) {
                            let method_info = method_info.clone();
                            self.check_method_args(
                                &class_name,
                                &method_info,
                                arg_count,
                                method_span.start.offset as usize,
                            );
                        }
                    }
                }
                // Case 2: $obj->method() where $obj was assigned from new ClassName()
                else if let Expression::Variable(Variable::Direct(var)) = &*call.object {
                    let var_name = self.get_span_text(&var.span).to_string();
                    if let Some(class_name) = self.variable_types.get(&var_name).cloned() {
                        let class_lower = class_name.to_lowercase();
                        // Skip built-in classes
                        if self.builtin_classes.iter().any(|c| c.eq_ignore_ascii_case(&class_name)) {
                            return true;
                        }

                        // Check if method exists (local or cross-file)
                        if !self.method_exists(&class_name, &method) {
                            // Method not found anywhere
                            let (line, col) = self.get_line_col(method_span.start.offset as usize);
                            self.issues.push(
                                Issue::error(
                                    "method.notFound",
                                    format!(
                                        "Call to an undefined method {}::{}().",
                                        class_name, method
                                    ),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("method.notFound"),
                            );
                        } else if let Some(methods) = self.class_methods.get(&class_lower) {
                            // Method exists in local file - check argument count
                            if let Some(method_info) = methods.get(&method_lower) {
                                let method_info = method_info.clone();
                                self.check_method_args(
                                    &class_name,
                                    &method_info,
                                    arg_count,
                                    method_span.start.offset as usize,
                                );
                            }
                        }
                    }
                }
            }
        }
        true
    }
}

impl<'s> MethodCallVisitor<'s> {
    fn check_method_args(
        &mut self,
        class_name: &str,
        method_info: &MethodInfo,
        arg_count: usize,
        offset: usize,
    ) {
        if !self.check_arg_counts {
            return;
        }

        let (line, col) = self.get_line_col(offset);

        // Check for too few arguments
        if arg_count < method_info.min_params {
            self.issues.push(
                Issue::error(
                    "arguments.count",
                    format!(
                        "Method {}::{}() invoked with {} {}, {} required.",
                        class_name,
                        method_info.name,
                        arg_count,
                        if arg_count == 1 { "parameter" } else { "parameters" },
                        method_info.min_params
                    ),
                    self.file_path.clone(),
                    line,
                    col,
                )
                .with_identifier("arguments.count"),
            );
            return;
        }

        // Check for too many arguments
        if let Some(max) = method_info.max_params {
            if arg_count > max {
                self.issues.push(
                    Issue::error(
                        "arguments.count",
                        format!(
                            "Method {}::{}() invoked with {} {}, {} required.",
                            class_name,
                            method_info.name,
                            arg_count,
                            if arg_count == 1 { "parameter" } else { "parameters" },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_check_level() {
        let check = CallMethodsCheck;
        assert_eq!(check.level(), 2);
    }
}
