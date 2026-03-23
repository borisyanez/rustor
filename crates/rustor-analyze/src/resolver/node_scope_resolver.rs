//! Node scope resolver for type-aware AST traversal
//!
//! Walks the PHP AST while maintaining scope information and running checks.

use crate::checks::{Check, CheckContext};
use crate::config::PhpStanConfig;
use crate::issue::Issue;
use crate::scope::{Scope, ClassContext, FunctionContext, ParameterInfo};
use crate::symbols::SymbolTable;
use crate::symbols::class_info::ClassMethodInfo;
use crate::types::Type;
use crate::resolver::expression_resolver::ExpressionResolver;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::path::Path;

/// Node scope resolver that traverses AST with scope tracking
pub struct NodeScopeResolver<'a> {
    symbol_table: &'a SymbolTable,
    config: &'a PhpStanConfig,
    source: &'a str,
    file_path: &'a Path,
    expression_resolver: ExpressionResolver<'a>,
}

impl<'a> NodeScopeResolver<'a> {
    /// Create a new node scope resolver
    pub fn new(
        symbol_table: &'a SymbolTable,
        config: &'a PhpStanConfig,
        source: &'a str,
        file_path: &'a Path,
    ) -> Self {
        Self {
            symbol_table,
            config,
            source,
            file_path,
            expression_resolver: ExpressionResolver::new(symbol_table, source),
        }
    }

    /// Analyze a program with scope tracking
    pub fn analyze(
        &self,
        program: &Program<'_>,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
    ) -> Vec<Issue> {
        let mut issues = Vec::new();
        let mut scope = Scope::new();

        // Check for strict_types declaration
        for statement in program.statements.iter() {
            if let Statement::Declare(declare) = statement {
                for entry in declare.items.iter() {
                    let name = self.get_span_text(&entry.name.span());
                    if name == "strict_types" {
                        if let Expression::Literal(Literal::Integer(i)) = &entry.value {
                            let val = self.get_span_text(&i.span());
                            scope.set_strict_types(val == "1");
                        }
                    }
                }
            }
        }

        // Process all statements
        for statement in program.statements.iter() {
            self.process_statement(statement, &mut scope, checks, ctx, &mut issues);
        }

        issues
    }

    /// Process a statement
    fn process_statement(
        &self,
        stmt: &Statement<'_>,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        match stmt {
            Statement::Namespace(ns) => {
                self.process_namespace(ns, scope, checks, ctx, issues);
            }
            Statement::Use(use_stmt) => {
                self.process_use(use_stmt, scope);
            }
            Statement::Class(class) => {
                self.process_class(class, scope, checks, ctx, issues);
            }
            Statement::Interface(interface) => {
                self.process_interface(interface, scope, checks, ctx, issues);
            }
            Statement::Trait(trait_def) => {
                self.process_trait(trait_def, scope, checks, ctx, issues);
            }
            Statement::Enum(enum_def) => {
                self.process_enum(enum_def, scope, checks, ctx, issues);
            }
            Statement::Function(func) => {
                self.process_function(func, scope, checks, ctx, issues);
            }
            Statement::Expression(expr_stmt) => {
                self.process_expression(&expr_stmt.expression, scope, issues);
            }
            Statement::If(if_stmt) => {
                self.process_if(if_stmt, scope, checks, ctx, issues);
            }
            Statement::While(while_stmt) => {
                self.process_while(while_stmt, scope, checks, ctx, issues);
            }
            Statement::DoWhile(do_while) => {
                self.process_do_while(do_while, scope, checks, ctx, issues);
            }
            Statement::For(for_stmt) => {
                self.process_for(for_stmt, scope, checks, ctx, issues);
            }
            Statement::Foreach(foreach) => {
                self.process_foreach(foreach, scope, checks, ctx, issues);
            }
            Statement::Switch(switch) => {
                self.process_switch(switch, scope, checks, ctx, issues);
            }
            Statement::Try(try_stmt) => {
                self.process_try(try_stmt, scope, checks, ctx, issues);
            }
            Statement::Block(block) => {
                for inner_stmt in block.statements.iter() {
                    self.process_statement(inner_stmt, scope, checks, ctx, issues);
                }
            }
            Statement::Return(ret) => {
                if let Some(expr) = &ret.value {
                    self.process_expression(expr, scope, issues);
                }
            }
            Statement::Global(global) => {
                // Add global variables to scope
                for var in global.variables.iter() {
                    let span = var.span();
                    let name = self.get_span_text(&span)
                        .trim_start_matches('$');
                    scope.set_variable(name.to_string(), Type::Mixed);
                }
            }
            _ => {}
        }
    }

    /// Process namespace
    fn process_namespace(
        &self,
        ns: &Namespace,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        match &ns.body {
            NamespaceBody::Implicit(body) => {
                // Extract namespace name from text
                let ns_span = ns.span();
                let ns_text = self.get_span_text(&ns_span);
                if let Some(name_start) = ns_text.find("namespace") {
                    let after_keyword = &ns_text[name_start + 9..];
                    let name_end = after_keyword.find(|c: char| c == '{' || c == ';')
                        .unwrap_or(after_keyword.len());
                    let name = after_keyword[..name_end].trim();
                    if !name.is_empty() {
                        scope.set_namespace(name.to_string());
                    }
                }
                for stmt in body.statements.iter() {
                    self.process_statement(stmt, scope, checks, ctx, issues);
                }
            }
            NamespaceBody::BraceDelimited(braced) => {
                let mut inner_scope = scope.enter_scope();
                let ns_span = ns.span();
                let ns_text = self.get_span_text(&ns_span);
                if let Some(name_start) = ns_text.find("namespace") {
                    let after_keyword = &ns_text[name_start + 9..];
                    let name_end = after_keyword.find(|c: char| c == '{' || c == ';')
                        .unwrap_or(after_keyword.len());
                    let name = after_keyword[..name_end].trim();
                    if !name.is_empty() {
                        inner_scope.set_namespace(name.to_string());
                    }
                }
                for stmt in braced.statements.iter() {
                    self.process_statement(stmt, &mut inner_scope, checks, ctx, issues);
                }
            }
        }
    }

    /// Process use statement - extract imports from text
    fn process_use(&self, use_stmt: &Use, scope: &mut Scope) {
        let use_span = use_stmt.span();
        let use_text = self.get_span_text(&use_span);

        // Simple text-based extraction of use imports
        let text = use_text.trim().trim_start_matches("use").trim();
        let text = text.trim_end_matches(';').trim();

        // Handle grouped use: use Namespace\{A, B as C};
        if let Some(brace_start) = text.find('{') {
            let prefix = text[..brace_start].trim().trim_end_matches('\\');
            let inner = text[brace_start + 1..].trim_end_matches('}').trim();
            for item in inner.split(',') {
                let item = item.trim();
                if item.is_empty() { continue; }
                let (name, alias) = if let Some(as_pos) = item.find(" as ") {
                    let name = item[..as_pos].trim();
                    let alias = item[as_pos + 4..].trim();
                    (name, alias.to_string())
                } else {
                    let alias = item.rsplit('\\').next().unwrap_or(item);
                    (item, alias.to_string())
                };
                let full_name = format!("{}\\{}", prefix, name);
                scope.add_use_import(alias, full_name);
            }
        } else {
            // Handle simple use: use Namespace\Class; or use Namespace\Class as Alias;
            for item in text.split(',') {
                let item = item.trim();
                if item.is_empty() { continue; }
                let (name, alias) = if let Some(as_pos) = item.find(" as ") {
                    let name = item[..as_pos].trim();
                    let alias = item[as_pos + 4..].trim();
                    (name, alias.to_string())
                } else {
                    let alias = item.rsplit('\\').next().unwrap_or(item);
                    (item, alias.to_string())
                };
                scope.add_use_import(alias, name.to_string());
            }
        }
    }

    /// Process class
    fn process_class(
        &self,
        class: &Class,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&class.name.span);
        let full_name = scope.resolve_class_name(&name);

        let mut class_ctx = ClassContext::new(&full_name);
        class_ctx.is_abstract = class.modifiers.contains_abstract();
        class_ctx.is_final = class.modifiers.contains_final();

        if let Some(extends) = &class.extends {
            if let Some(parent) = extends.types.first() {
                let parent_text = self.get_span_text(&parent.span());
                class_ctx.parent = Some(parent_text.to_string());
            }
        }

        let mut class_scope = scope.enter_class_scope(class_ctx);

        for member in class.members.iter() {
            self.process_class_member(member, &mut class_scope, checks, ctx, issues);
        }
    }

    /// Process interface
    fn process_interface(
        &self,
        interface: &Interface,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&interface.name.span);
        let full_name = scope.resolve_class_name(&name);

        let mut class_ctx = ClassContext::new(&full_name);
        class_ctx.is_interface = true;

        let mut _class_scope = scope.enter_class_scope(class_ctx);

        for member in interface.members.iter() {
            match member {
                ClassLikeMember::Method(_method) => {
                    // Interface methods don't have bodies
                }
                _ => {}
            }
        }
    }

    /// Process trait
    fn process_trait(
        &self,
        trait_def: &Trait,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&trait_def.name.span);
        let full_name = scope.resolve_class_name(&name);

        let mut class_ctx = ClassContext::new(&full_name);
        class_ctx.is_trait = true;

        let mut class_scope = scope.enter_class_scope(class_ctx);

        for member in trait_def.members.iter() {
            self.process_class_member(member, &mut class_scope, checks, ctx, issues);
        }
    }

    /// Process enum
    fn process_enum(
        &self,
        enum_def: &Enum,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&enum_def.name.span);
        let full_name = scope.resolve_class_name(&name);

        let mut class_ctx = ClassContext::new(&full_name);
        class_ctx.is_enum = true;

        let mut class_scope = scope.enter_class_scope(class_ctx);

        for member in enum_def.members.iter() {
            match member {
                ClassLikeMember::Method(_) | ClassLikeMember::Property(_) | ClassLikeMember::Constant(_) => {
                    self.process_class_member(member, &mut class_scope, checks, ctx, issues);
                }
                _ => {}
            }
        }
    }

    /// Process class member
    fn process_class_member(
        &self,
        member: &ClassLikeMember,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        match member {
            ClassLikeMember::Method(method) => {
                self.process_method(method, scope, checks, ctx, issues);
            }
            ClassLikeMember::Property(Property::Plain(prop)) => {
                // Properties with default values
                for item in prop.items.nodes.iter() {
                    if let PropertyItem::Concrete(init) = item {
                        self.process_expression(&init.value, scope, issues);
                    }
                }
            }
            _ => {}
        }
    }

    /// Process method
    fn process_method(
        &self,
        method: &Method,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&method.name.span);
        let is_static = method.modifiers.iter().any(|m| matches!(m, Modifier::Static(_)));

        let mut func_ctx = FunctionContext::new(name)
            .with_method(true)
            .with_static(is_static);

        // Add parameters
        for param in method.parameter_list.parameters.iter() {
            let param_name = self.get_span_text(&param.variable.span())
                .trim_start_matches('$');
            let param_type = param.hint.as_ref()
                .map(|h| self.expression_resolver.resolve_type_hint(h, scope))
                .unwrap_or(Type::Mixed);

            func_ctx = func_ctx.with_parameter(
                ParameterInfo::new(param_name)
                    .with_type(param_type)
                    .with_optional(param.default_value.is_some())
                    .with_variadic(param.ellipsis.is_some())
            );
        }

        // Return type
        if let Some(return_hint) = &method.return_type_hint {
            let return_type = self.expression_resolver.resolve_type_hint(&return_hint.hint, scope);
            func_ctx = func_ctx.with_return_type(return_type);
        }

        let mut method_scope = scope.enter_function_scope(func_ctx);

        // Add $this with the class type (for non-static methods)
        if !is_static {
            if let Some(class_ctx) = scope.class_context() {
                method_scope.set_variable("this".to_string(), Type::Object {
                    class_name: Some(class_ctx.name.clone()),
                });
            }
        }

        // Process method body
        if let MethodBody::Concrete(body) = &method.body {
            for stmt in body.statements.iter() {
                self.process_statement(stmt, &mut method_scope, checks, ctx, issues);
            }
        }
    }

    /// Process function
    fn process_function(
        &self,
        func: &Function,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&func.name.span);
        let mut func_ctx = FunctionContext::new(name);

        // Add parameters
        for param in func.parameter_list.parameters.iter() {
            let param_name = self.get_span_text(&param.variable.span())
                .trim_start_matches('$');
            let param_type = param.hint.as_ref()
                .map(|h| self.expression_resolver.resolve_type_hint(h, scope))
                .unwrap_or(Type::Mixed);

            func_ctx = func_ctx.with_parameter(
                ParameterInfo::new(param_name)
                    .with_type(param_type)
                    .with_optional(param.default_value.is_some())
                    .with_variadic(param.ellipsis.is_some())
            );
        }

        // Return type
        if let Some(return_hint) = &func.return_type_hint {
            let return_type = self.expression_resolver.resolve_type_hint(&return_hint.hint, scope);
            func_ctx = func_ctx.with_return_type(return_type);
        }

        let mut func_scope = scope.enter_function_scope(func_ctx);

        // Process function body
        for stmt in func.body.statements.iter() {
            self.process_statement(stmt, &mut func_scope, checks, ctx, issues);
        }
    }

    /// Process expression and track variable assignments
    fn process_expression(
        &self,
        expr: &Expression<'_>,
        scope: &mut Scope,
        _issues: &mut Vec<Issue>,
    ) {
        match expr {
            Expression::Assignment(assign) => {
                // Track variable type from assignment
                if let Expression::Variable(Variable::Direct(direct)) = &assign.lhs {
                    let var_name = self.get_span_text(&direct.span())
                        .trim_start_matches('$');
                    let rhs_type = self.expression_resolver.resolve(&assign.rhs, scope);
                    scope.set_variable(var_name.to_string(), rhs_type);
                }
                // Process RHS
                self.process_expression(&assign.rhs, scope, _issues);
            }
            Expression::Closure(closure) => {
                // Create closure scope with use bindings
                let mut bindings = std::collections::HashSet::new();
                if let Some(use_clause) = &closure.use_clause {
                    for item in use_clause.variables.iter() {
                        let name = self.get_span_text(&item.variable.span())
                            .trim_start_matches('$');
                        bindings.insert(name.to_string());
                    }
                }
                let mut closure_scope = scope.enter_closure_scope(bindings);

                // Add parameters
                for param in closure.parameter_list.parameters.iter() {
                    let param_name = self.get_span_text(&param.variable.span())
                        .trim_start_matches('$');
                    let param_type = param.hint.as_ref()
                        .map(|h| self.expression_resolver.resolve_type_hint(h, scope))
                        .unwrap_or(Type::Mixed);
                    closure_scope.set_variable(param_name.to_string(), param_type);
                }

                // Process body
                for stmt in closure.body.statements.iter() {
                    self.process_statement(stmt, &mut closure_scope, &[], &CheckContext {
                        file_path: self.file_path,
                        source: self.source,
                        config: self.config,
                        builtin_functions: &[],
                        builtin_classes: &[],
                        symbol_table: None,
                        scope: None,
                        analysis_level: self.config.level.as_u8(),
                    }, _issues);
                }
            }
            // Track method call expressions and check arg counts + arg types
            Expression::Call(Call::Method(method_call)) => {
                let obj_type = self.expression_resolver.resolve(&method_call.object, scope);
                if let Some(class_name) = obj_type.get_class_name() {
                    if let ClassLikeMemberSelector::Identifier(ident) = &method_call.method {
                        let method_name = self.get_span_text(&ident.span);
                        let arg_count = method_call.argument_list.arguments.len();
                        // Check arg count
                        self.check_method_arg_count(class_name, method_name, arg_count,
                            method_call.argument_list.span().start.offset as usize, _issues);
                        // Check arg types (level 5+)
                        if self.config.level.as_u8() >= 5 {
                            self.check_method_arg_types(class_name, method_name,
                                &method_call.argument_list, scope, _issues);
                        }
                    }
                }
                for arg in method_call.argument_list.arguments.iter() {
                    self.process_expression(arg.value(), scope, _issues);
                }
            }
            // Track static method calls and check arg types
            Expression::Call(Call::StaticMethod(static_call)) => {
                let class_name = self.resolve_static_class(&static_call.class, scope);
                if !class_name.is_empty() {
                    if let ClassLikeMemberSelector::Identifier(ident) = &static_call.method {
                        let method_name = self.get_span_text(&ident.span);
                        let arg_count = static_call.argument_list.arguments.len();
                        // Check arg count
                        self.check_method_arg_count(&class_name, method_name, arg_count,
                            static_call.argument_list.span().start.offset as usize, _issues);
                        // Check arg types (level 5+)
                        if self.config.level.as_u8() >= 5 {
                            self.check_method_arg_types(&class_name, method_name,
                                &static_call.argument_list, scope, _issues);
                        }
                    }
                }
                for arg in static_call.argument_list.arguments.iter() {
                    self.process_expression(arg.value(), scope, _issues);
                }
            }
            // Check function call argument types using scope
            Expression::Call(Call::Function(func_call)) => {
                if self.config.level.as_u8() >= 5 {
                    self.check_function_arg_types(func_call, scope, _issues);
                }
                for arg in func_call.argument_list.arguments.iter() {
                    self.process_expression(arg.value(), scope, _issues);
                }
            }
            // Process instantiation constructor arguments
            Expression::Instantiation(instantiation) => {
                for arg in instantiation.argument_list.iter() {
                    for a in arg.arguments.iter() {
                        self.process_expression(a.value(), scope, _issues);
                    }
                }
            }
            // Process ternary/conditional expressions
            Expression::Conditional(conditional) => {
                self.process_expression(&conditional.condition, scope, _issues);
                if let Some(ref then_expr) = conditional.then {
                    self.process_expression(then_expr, scope, _issues);
                }
                self.process_expression(&conditional.r#else, scope, _issues);
            }
            // Process parenthesized expressions
            Expression::Parenthesized(paren) => {
                self.process_expression(&paren.expression, scope, _issues);
            }
            _ => {}
        }
    }

    /// Check argument types for a function call using scope-resolved types
    fn check_function_arg_types(
        &self,
        func_call: &FunctionCall<'_>,
        scope: &Scope,
        issues: &mut Vec<Issue>,
    ) {
        // Get function name
        let func_span = func_call.function.span();
        let func_name = self.get_span_text(&func_span);

        // Skip dynamic/variable calls
        if func_name.starts_with('$') || func_name.is_empty() {
            return;
        }

        // Resolve function name
        let resolved_name = scope.resolve_class_name(func_name);

        // Look up function in symbol table
        if let Some(func_info) = self.symbol_table.get_function(&resolved_name)
            .or_else(|| self.symbol_table.get_function(func_name))
        {
            if func_info.parameters.is_empty() {
                return; // No param info
            }

            // Skip named arguments
            if func_call.argument_list.arguments.iter().any(|a| matches!(a, Argument::Named(_))) {
                return;
            }

            for (i, arg) in func_call.argument_list.arguments.iter().enumerate() {
                if let Some(param) = func_info.parameters.get(i) {
                    if let Some(ref expected_type) = param.type_ {
                        // Resolve actual argument type using ExpressionResolver with scope
                        let actual_type = self.expression_resolver.resolve(arg.value(), scope);

                        if !matches!(actual_type, Type::Mixed | Type::Null)
                            && !matches!(&actual_type, Type::Object { class_name: None })
                        {
                            // Use hierarchy-aware accepts
                            let result = expected_type.accepts_with_hierarchy(
                                &actual_type, false, self.symbol_table
                            );
                            if result.no() {
                                let (line, col) = self.get_line_col(arg.span().start.offset as usize);
                                let expected_str = self.type_to_display_string(expected_type);
                                let actual_str = self.type_to_display_string(&actual_type);
                                issues.push(
                                    crate::issue::Issue::error(
                                        "argument.type",
                                        format!(
                                            "Parameter #{} ${} of function {} expects {}, {} given.",
                                            i + 1, param.name, func_name, expected_str, actual_str
                                        ),
                                        self.file_path.to_path_buf(),
                                        line,
                                        col,
                                    )
                                    .with_identifier("argument.type"),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check argument count for a method call
    fn check_method_arg_count(
        &self,
        class_name: &str,
        method_name: &str,
        arg_count: usize,
        offset: usize,
        issues: &mut Vec<Issue>,
    ) {
        if let Some(class_info) = self.symbol_table.get_class(class_name) {
            if let Some(method_info) = class_info.get_method(method_name) {
                // Skip if no parameter info (cache stub)
                if method_info.parameters.is_empty() && arg_count > 0 {
                    return;
                }
                let required = method_info.required_args();
                let max = method_info.max_args();

                if arg_count < required {
                    let (line, col) = self.get_line_col(offset);
                    let range = if let Some(m) = max {
                        if required == m { format!("{}", required) } else { format!("{}-{}", required, m) }
                    } else { format!("{}", required) };
                    issues.push(
                        crate::issue::Issue::error(
                            "arguments.count",
                            format!(
                                "Method {}::{}() invoked with {} parameter{}, {} required.",
                                class_name, method_name, arg_count,
                                if arg_count == 1 { "" } else { "s" }, range
                            ),
                            self.file_path.to_path_buf(),
                            line, col,
                        )
                        .with_identifier("arguments.count"),
                    );
                } else if let Some(max_count) = max {
                    if arg_count > max_count {
                        let (line, col) = self.get_line_col(offset);
                        let range = if required == max_count { format!("{}", max_count) } else { format!("{}-{}", required, max_count) };
                        issues.push(
                            crate::issue::Issue::error(
                                "arguments.count",
                                format!(
                                    "Method {}::{}() invoked with {} parameter{}, {} required.",
                                    class_name, method_name, arg_count,
                                    if arg_count == 1 { "" } else { "s" }, range
                                ),
                                self.file_path.to_path_buf(),
                                line, col,
                            )
                            .with_identifier("arguments.count"),
                        );
                    }
                }
            }
        }
    }

    /// Resolve the class name from a static call target expression
    fn resolve_static_class(&self, expr: &Expression<'_>, scope: &Scope) -> String {
        match expr {
            Expression::Identifier(ident) => {
                let name = self.get_span_text(&ident.span());
                scope.resolve_class_name(name)
            }
            Expression::Self_(_) => {
                scope.class_context().map_or(String::new(), |c| c.name.clone())
            }
            Expression::Static(_) => {
                scope.class_context().map_or("static".to_string(), |c| c.name.clone())
            }
            Expression::Parent(_) => {
                scope.class_context()
                    .and_then(|c| c.parent.clone())
                    .unwrap_or_else(|| "parent".to_string())
            }
            _ => String::new(),
        }
    }

    /// Check argument types for a method call using scope-resolved types
    fn check_method_arg_types(
        &self,
        class_name: &str,
        method_name: &str,
        arg_list: &ArgumentList<'_>,
        scope: &Scope,
        issues: &mut Vec<Issue>,
    ) {
        // Skip named arguments
        if arg_list.arguments.iter().any(|a| matches!(a, Argument::Named(_))) {
            return;
        }

        // Skip test files (mock objects cause false positives)
        let path_str = self.file_path.to_string_lossy();
        if path_str.contains("test/") || path_str.contains("Test.") || path_str.contains("Tests/") {
            return;
        }

        // Look up method in hierarchy
        if let Some((_found_class, method)) = self.find_method_in_hierarchy(class_name, method_name) {
            // Skip if no parameter info (cache stub)
            if method.parameters.is_empty() {
                return;
            }

            for (i, arg) in arg_list.arguments.iter().enumerate() {
                if let Some(param) = method.parameters.get(i) {
                    if let Some(ref expected_type) = param.type_ {
                        let actual_type = self.expression_resolver.resolve(arg.value(), scope);

                        // Skip Mixed, Null (PHPStan uses narrowing), and generic Object
                        if !matches!(actual_type, Type::Mixed | Type::Null)
                            && !matches!(&actual_type, Type::Object { class_name: None })
                        {
                            let result = expected_type.accepts_with_hierarchy(
                                &actual_type, false, self.symbol_table
                            );
                            if result.no() {
                                let (line, col) = self.get_line_col(arg.span().start.offset as usize);
                                let expected_str = self.type_to_display_string(expected_type);
                                let actual_str = self.type_to_display_string(&actual_type);
                                issues.push(
                                    crate::issue::Issue::error(
                                        "argument.type",
                                        format!(
                                            "Parameter #{} ${} of method {}::{}() expects {}, {} given.",
                                            i + 1, param.name, class_name, method_name,
                                            expected_str, actual_str
                                        ),
                                        self.file_path.to_path_buf(),
                                        line,
                                        col,
                                    )
                                    .with_identifier("argument.type"),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Find a method in the class hierarchy (class, parent, interfaces)
    fn find_method_in_hierarchy<'b>(&'b self, class_name: &str, method_name: &str) -> Option<(String, &'b ClassMethodInfo)> {
        if let Some(class_info) = self.symbol_table.get_class(class_name) {
            if let Some(method) = class_info.get_method(method_name) {
                return Some((class_name.to_string(), method));
            }
            // Check parent
            if let Some(parent) = &class_info.parent {
                let parent = parent.clone();
                if let Some(result) = self.find_method_in_hierarchy(&parent, method_name) {
                    return Some(result);
                }
            }
            // Check traits
            let traits: Vec<String> = class_info.traits.clone();
            for trait_name in &traits {
                if let Some(result) = self.find_method_in_hierarchy(trait_name, method_name) {
                    return Some(result);
                }
            }
            // Check interfaces
            let interfaces: Vec<String> = class_info.interfaces.clone();
            for iface in &interfaces {
                if let Some(result) = self.find_method_in_hierarchy(iface, method_name) {
                    return Some(result);
                }
            }
        }
        None
    }

    fn type_to_display_string(&self, ty: &Type) -> String {
        match ty {
            Type::Int | Type::ConstantInt(_) => "int".to_string(),
            Type::String | Type::ConstantString(_) => "string".to_string(),
            Type::Float | Type::ConstantFloat(_) => "float".to_string(),
            Type::Bool => "bool".to_string(),
            Type::ConstantBool(true) => "true".to_string(),
            Type::ConstantBool(false) => "false".to_string(),
            Type::Null => "null".to_string(),
            Type::Void => "void".to_string(),
            Type::Mixed => "mixed".to_string(),
            Type::Object { class_name: Some(name) } => name.clone(),
            Type::Object { class_name: None } => "object".to_string(),
            Type::Nullable(inner) => format!("?{}", self.type_to_display_string(inner)),
            Type::Union(types) => types.iter().map(|t| self.type_to_display_string(t)).collect::<Vec<_>>().join("|"),
            Type::Array { .. } => "array".to_string(),
            Type::Callable | Type::Closure => "callable".to_string(),
            _ => "mixed".to_string(),
        }
    }

    fn get_line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset { break; }
            if ch == '\n' { line += 1; col = 1; } else { col += 1; }
        }
        (line, col)
    }

    /// Check if a method exists in a class hierarchy
    fn method_exists_in_hierarchy(&self, class_name: &str, method_name: &str) -> bool {
        if self.symbol_table.class_has_method(class_name, method_name) {
            return true;
        }
        // Check __call magic method
        if self.symbol_table.class_has_method(class_name, "__call") {
            return true;
        }
        if let Some(class_info) = self.symbol_table.get_class(class_name) {
            // Check parent
            if let Some(parent) = &class_info.parent {
                if self.method_exists_in_hierarchy(parent, method_name) {
                    return true;
                }
            }
            // Check traits
            for trait_name in &class_info.traits {
                if self.method_exists_in_hierarchy(trait_name, method_name) {
                    return true;
                }
            }
            // Check interfaces
            for iface in &class_info.interfaces {
                if self.method_exists_in_hierarchy(iface, method_name) {
                    return true;
                }
            }
        }
        false
    }

    /// Narrow scope types based on a condition expression
    /// `negated` indicates if this is for the else-branch (inverted condition)
    fn narrow_scope_from_condition(&self, condition: &Expression<'_>, scope: &mut Scope, negated: bool) {
        match condition {
            // $x !== null → $x is non-null in if branch
            Expression::Binary(binary) => {
                let is_not_identical = matches!(binary.operator, BinaryOperator::NotIdentical(_));
                let is_identical = matches!(binary.operator, BinaryOperator::Identical(_));
                let is_not_equal = matches!(binary.operator, BinaryOperator::NotEqual(_));
                let is_instanceof = matches!(binary.operator, BinaryOperator::Instanceof(_));

                if is_not_identical || is_identical || is_not_equal {
                    // Check for null comparison: $x !== null or $x === null
                    let (var_expr, is_null_check) = if Self::is_null_expr(&binary.rhs) {
                        (Some(&*binary.lhs), true)
                    } else if Self::is_null_expr(&binary.lhs) {
                        (Some(&*binary.rhs), true)
                    } else {
                        (None, false)
                    };

                    if is_null_check {
                        if let Some(Expression::Variable(Variable::Direct(var))) = var_expr {
                            let var_name = self.get_span_text(&var.span())
                                .trim_start_matches('$');
                            let removes_null = (is_not_identical || is_not_equal) != negated;
                            if removes_null {
                                // Remove null from the variable's type
                                if let Some(current_type) = scope.get_variable_type(var_name) {
                                    let narrowed = current_type.remove_null();
                                    scope.set_variable(var_name.to_string(), narrowed);
                                }
                            }
                        }
                    }
                }

                if is_instanceof && !negated {
                    // $x instanceof Foo → $x is Foo in if branch
                    if let Expression::Variable(Variable::Direct(var)) = &*binary.lhs {
                        if let Expression::Identifier(ident) = &*binary.rhs {
                            let var_name = self.get_span_text(&var.span())
                                .trim_start_matches('$');
                            let class_name = self.get_span_text(&ident.span());
                            let resolved = scope.resolve_class_name(class_name);
                            scope.set_variable(var_name.to_string(), Type::Object {
                                class_name: Some(resolved),
                            });
                        }
                    }
                }
            }
            // !$expr → negate the inner condition
            Expression::UnaryPrefix(unary) if matches!(unary.operator, UnaryPrefixOperator::Not(_)) => {
                self.narrow_scope_from_condition(&unary.operand, scope, !negated);
            }
            // is_string($x), is_int($x), etc.
            Expression::Call(Call::Function(func_call)) => {
                let func_span = func_call.function.span();
                let func_name = self.get_span_text(&func_span).to_lowercase();

                if !negated {
                    if let Some(arg) = func_call.argument_list.arguments.first() {
                        if let Expression::Variable(Variable::Direct(var)) = arg.value() {
                            let var_name = self.get_span_text(&var.span())
                                .trim_start_matches('$');
                            let narrowed_type = match func_name.as_str() {
                                "is_string" => Some(Type::String),
                                "is_int" | "is_integer" | "is_long" => Some(Type::Int),
                                "is_float" | "is_double" => Some(Type::Float),
                                "is_bool" => Some(Type::Bool),
                                "is_array" => Some(Type::mixed_array()),
                                "is_null" => Some(Type::Null),
                                "is_object" => Some(Type::Object { class_name: None }),
                                "is_callable" => Some(Type::Callable),
                                _ => None,
                            };
                            if let Some(ty) = narrowed_type {
                                scope.set_variable(var_name.to_string(), ty);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn is_null_expr(expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::Literal(Literal::Null(_)))
    }

    /// Process if statement
    fn process_if(
        &self,
        if_stmt: &If,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        // Process condition
        self.process_expression(&if_stmt.condition, scope, issues);

        // Process body with type narrowing from condition
        let mut if_scope = scope.enter_scope();
        self.narrow_scope_from_condition(&if_stmt.condition, &mut if_scope, false);
        match &if_stmt.body {
            IfBody::Statement(stmt_body) => {
                self.process_statement(stmt_body.statement, &mut if_scope, checks, ctx, issues);

                // Process elseif clauses
                for elseif in stmt_body.else_if_clauses.iter() {
                    self.process_expression(&elseif.condition, scope, issues);
                    let mut elseif_scope = scope.enter_scope();
                    self.process_statement(elseif.statement, &mut elseif_scope, checks, ctx, issues);
                }

                // Process else clause
                if let Some(else_clause) = &stmt_body.else_clause {
                    let mut else_scope = scope.enter_scope();
                    self.process_statement(else_clause.statement, &mut else_scope, checks, ctx, issues);
                }
            }
            IfBody::ColonDelimited(body) => {
                for stmt in body.statements.iter() {
                    self.process_statement(stmt, &mut if_scope, checks, ctx, issues);
                }

                // Process elseif clauses
                for elseif in body.else_if_clauses.iter() {
                    self.process_expression(&elseif.condition, scope, issues);
                    let mut elseif_scope = scope.enter_scope();
                    for stmt in elseif.statements.iter() {
                        self.process_statement(stmt, &mut elseif_scope, checks, ctx, issues);
                    }
                }

                // Process else clause
                if let Some(else_clause) = &body.else_clause {
                    let mut else_scope = scope.enter_scope();
                    for stmt in else_clause.statements.iter() {
                        self.process_statement(stmt, &mut else_scope, checks, ctx, issues);
                    }
                }
            }
        }
    }

    /// Process while statement
    fn process_while(
        &self,
        while_stmt: &While,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        self.process_expression(&while_stmt.condition, scope, issues);
        let mut loop_scope = scope.enter_scope();
        self.narrow_scope_from_condition(&while_stmt.condition, &mut loop_scope, false);
        match &while_stmt.body {
            WhileBody::Statement(stmt) => {
                self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
            }
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
                }
            }
        }
    }

    /// Process do-while statement
    fn process_do_while(
        &self,
        do_while: &DoWhile,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let mut loop_scope = scope.enter_scope();
        self.process_statement(&do_while.statement, &mut loop_scope, checks, ctx, issues);
        self.process_expression(&do_while.condition, scope, issues);
    }

    /// Process for statement
    fn process_for(
        &self,
        for_stmt: &For,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let mut loop_scope = scope.enter_scope();

        // Process initializations
        for expr in for_stmt.initializations.iter() {
            self.process_expression(expr, &mut loop_scope, issues);
        }

        // Process conditions
        for expr in for_stmt.conditions.iter() {
            self.process_expression(expr, &mut loop_scope, issues);
        }

        // Process increments
        for expr in for_stmt.increments.iter() {
            self.process_expression(expr, &mut loop_scope, issues);
        }

        // Process body
        match &for_stmt.body {
            ForBody::Statement(stmt) => {
                self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
            }
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
                }
            }
        }
    }

    /// Process foreach statement
    fn process_foreach(
        &self,
        foreach: &Foreach,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let mut loop_scope = scope.enter_scope();

        // Infer value type from expression
        let expr_type = self.expression_resolver.resolve(&foreach.expression, scope);
        let value_type = match expr_type {
            Type::Array { value, .. } | Type::List { value } | Type::NonEmptyArray { value, .. } => {
                *value
            }
            Type::Iterable { value, .. } => *value,
            _ => Type::Mixed,
        };

        // Add value variable to scope
        match &foreach.target {
            ForeachTarget::Value(value) => {
                if let Expression::Variable(Variable::Direct(direct)) = &value.value {
                    let name = self.get_span_text(&direct.span()).trim_start_matches('$');
                    loop_scope.set_variable(name.to_string(), value_type);
                }
            }
            ForeachTarget::KeyValue(kv) => {
                if let Expression::Variable(Variable::Direct(direct)) = &kv.key {
                    let name = self.get_span_text(&direct.span()).trim_start_matches('$');
                    loop_scope.set_variable(name.to_string(), Type::Mixed);
                }
                if let Expression::Variable(Variable::Direct(direct)) = &kv.value {
                    let name = self.get_span_text(&direct.span()).trim_start_matches('$');
                    loop_scope.set_variable(name.to_string(), value_type);
                }
            }
        }

        // Process body
        match &foreach.body {
            ForeachBody::Statement(stmt) => {
                self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
            }
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
                }
            }
        }
    }

    /// Process switch statement
    fn process_switch(
        &self,
        switch: &Switch,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        self.process_expression(&switch.expression, scope, issues);

        match &switch.body {
            SwitchBody::BraceDelimited(block) => {
                for case in block.cases.iter() {
                    let mut case_scope = scope.enter_scope();
                    for stmt in case.statements().iter() {
                        self.process_statement(stmt, &mut case_scope, checks, ctx, issues);
                    }
                }
            }
            SwitchBody::ColonDelimited(block) => {
                for case in block.cases.iter() {
                    let mut case_scope = scope.enter_scope();
                    for stmt in case.statements().iter() {
                        self.process_statement(stmt, &mut case_scope, checks, ctx, issues);
                    }
                }
            }
        }
    }

    /// Process try statement
    fn process_try(
        &self,
        try_stmt: &Try,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        // Process try body
        let mut try_scope = scope.enter_scope();
        for stmt in try_stmt.block.statements.iter() {
            self.process_statement(stmt, &mut try_scope, checks, ctx, issues);
        }

        // Process catch clauses
        for catch in try_stmt.catch_clauses.iter() {
            let mut catch_scope = scope.enter_scope();

            // Add exception variable
            if let Some(var) = &catch.variable {
                let name = self.get_span_text(&var.span()).trim_start_matches('$');
                // Use the hint to determine the exception type
                let exc_type = {
                    let hint_span = catch.hint.span();
                    let hint_text = self.get_span_text(&hint_span);
                    if hint_text.is_empty() {
                        Type::object("Throwable")
                    } else {
                        // Parse union types separated by |
                        let types: Vec<Type> = hint_text.split('|')
                            .map(|t| Type::object(t.trim()))
                            .collect();
                        if types.len() == 1 {
                            types.into_iter().next().unwrap()
                        } else {
                            Type::Union(types)
                        }
                    }
                };
                catch_scope.set_variable(name.to_string(), exc_type);
            }

            for stmt in catch.block.statements.iter() {
                self.process_statement(stmt, &mut catch_scope, checks, ctx, issues);
            }
        }

        // Process finally
        if let Some(finally) = &try_stmt.finally_clause {
            let mut finally_scope = scope.enter_scope();
            for stmt in finally.block.statements.iter() {
                self.process_statement(stmt, &mut finally_scope, checks, ctx, issues);
            }
        }
    }

    /// Get text for a span
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}
