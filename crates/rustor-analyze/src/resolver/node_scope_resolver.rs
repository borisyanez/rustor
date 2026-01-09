//! Node scope resolver for type-aware AST traversal
//!
//! Walks the PHP AST while maintaining scope information and running checks.

use crate::checks::{Check, CheckContext};
use crate::config::PhpStanConfig;
use crate::issue::Issue;
use crate::scope::{Scope, ClassContext, FunctionContext, ParameterInfo};
use crate::symbols::SymbolTable;
use crate::types::Type;
use crate::resolver::expression_resolver::ExpressionResolver;
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
            if let Statement::DeclareBlock(declare) = statement {
                for entry in declare.declare.entries.iter() {
                    let name = self.get_span_text(&entry.name.span);
                    if name == "strict_types" {
                        if let Expression::Literal(Literal::Integer(i)) = &entry.value {
                            let val = self.get_span_text(&i.token.span);
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
                    if let Variable::Direct(direct) = var {
                        let name = self.get_span_text(&direct.name.span)
                            .trim_start_matches('$');
                        scope.set_variable(name.to_string(), Type::Mixed);
                    }
                }
            }
            _ => {}
        }
    }

    /// Process namespace
    fn process_namespace(
        &self,
        ns: &NamespaceStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        match ns {
            NamespaceStatement::Unbraced(unbraced) => {
                let name = self.get_name_text(&unbraced.name);
                scope.set_namespace(name);
                for stmt in unbraced.statements.iter() {
                    self.process_statement(stmt, scope, checks, ctx, issues);
                }
            }
            NamespaceStatement::Braced(braced) => {
                let mut inner_scope = scope.enter_scope();
                if let Some(name) = &braced.name {
                    inner_scope.set_namespace(self.get_name_text(name));
                }
                for stmt in braced.statements.iter() {
                    self.process_statement(stmt, &mut inner_scope, checks, ctx, issues);
                }
            }
        }
    }

    /// Process use statement
    fn process_use(&self, use_stmt: &UseStatement, scope: &mut Scope) {
        match use_stmt {
            UseStatement::Default(default) => {
                for item in default.items.iter() {
                    match item {
                        UseItem::TypeAlias(alias) => {
                            let name = self.get_name_text(&alias.name);
                            let alias_name = alias.alias.as_ref()
                                .map(|a| self.get_span_text(&a.alias.span).to_string())
                                .unwrap_or_else(|| name.rsplit('\\').next().unwrap_or(&name).to_string());
                            scope.add_use_import(alias_name, name);
                        }
                        UseItem::TypeGroup(group) => {
                            let prefix = self.get_name_text(&group.namespace);
                            for item in group.items.iter() {
                                match item {
                                    UseGroupItem::Alias(alias) => {
                                        let name = self.get_span_text(&alias.name.span);
                                        let full_name = format!("{}\\{}", prefix, name);
                                        let alias_name = alias.alias.as_ref()
                                            .map(|a| self.get_span_text(&a.alias.span).to_string())
                                            .unwrap_or_else(|| name.to_string());
                                        scope.add_use_import(alias_name, full_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Process class
    fn process_class(
        &self,
        class: &ClassStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&class.name.span);
        let full_name = scope.resolve_class_name(&name);

        let mut class_ctx = ClassContext::new(&full_name);
        class_ctx.is_abstract = class.modifiers.iter().any(|m| matches!(m, ClassModifier::Abstract(_)));
        class_ctx.is_final = class.modifiers.iter().any(|m| matches!(m, ClassModifier::Final(_)));

        if let Some(extends) = &class.extends {
            class_ctx.parent = Some(self.get_name_text(&extends.parent));
        }

        let mut class_scope = scope.enter_class_scope(class_ctx);

        for member in class.body.members.iter() {
            self.process_class_member(member, &mut class_scope, checks, ctx, issues);
        }
    }

    /// Process interface
    fn process_interface(
        &self,
        interface: &InterfaceStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&interface.name.span);
        let full_name = scope.resolve_class_name(&name);

        let mut class_ctx = ClassContext::new(&full_name);
        class_ctx.is_interface = true;

        let mut class_scope = scope.enter_class_scope(class_ctx);

        for member in interface.body.members.iter() {
            match member {
                InterfaceMember::Method(method) => {
                    // Interface methods don't have bodies
                }
                _ => {}
            }
        }
    }

    /// Process trait
    fn process_trait(
        &self,
        trait_def: &TraitStatement,
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

        for member in trait_def.body.members.iter() {
            self.process_class_member(member, &mut class_scope, checks, ctx, issues);
        }
    }

    /// Process enum
    fn process_enum(
        &self,
        enum_def: &EnumStatement,
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

        for member in enum_def.body.members.iter() {
            match member {
                EnumMember::ClassLike(class_member) => {
                    self.process_class_member(class_member, &mut class_scope, checks, ctx, issues);
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
            ClassLikeMember::Property(prop) => {
                // Properties with default values
                for entry in prop.entries.iter() {
                    if let PropertyEntry::Initialized(init) = entry {
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
        method: &MethodStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&method.name.span);
        let is_static = method.modifiers.iter().any(|m| matches!(m, MethodModifier::Static(_)));

        let mut func_ctx = FunctionContext::new(&name)
            .with_method(true)
            .with_static(is_static);

        // Add parameters
        for param in method.parameters.parameters.iter() {
            let param_name = self.get_span_text(&param.variable.name.span)
                .trim_start_matches('$');
            let param_type = param.hint.as_ref()
                .map(|h| self.expression_resolver.resolve_type_hint(h, scope))
                .unwrap_or(Type::Mixed);

            func_ctx = func_ctx.with_parameter(
                ParameterInfo::new(param_name)
                    .with_type(param_type)
                    .with_optional(param.default.is_some())
                    .with_variadic(param.ellipsis.is_some())
            );
        }

        // Return type
        if let Some(return_hint) = &method.return_type_hint {
            let return_type = self.expression_resolver.resolve_type_hint(&return_hint.hint, scope);
            func_ctx = func_ctx.with_return_type(return_type);
        }

        let mut method_scope = scope.enter_function_scope(func_ctx);

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
        func: &FunctionStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let name = self.get_span_text(&func.name.span);
        let mut func_ctx = FunctionContext::new(&name);

        // Add parameters
        for param in func.parameters.parameters.iter() {
            let param_name = self.get_span_text(&param.variable.name.span)
                .trim_start_matches('$');
            let param_type = param.hint.as_ref()
                .map(|h| self.expression_resolver.resolve_type_hint(h, scope))
                .unwrap_or(Type::Mixed);

            func_ctx = func_ctx.with_parameter(
                ParameterInfo::new(param_name)
                    .with_type(param_type)
                    .with_optional(param.default.is_some())
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
            Expression::AssignmentOperation(assign) => {
                // Track variable type from assignment
                if let Expression::Variable(Variable::Direct(direct)) = &assign.lhs {
                    let var_name = self.get_span_text(&direct.name.span)
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
                    for item in use_clause.items.iter() {
                        let name = self.get_span_text(&item.variable.name.span)
                            .trim_start_matches('$');
                        bindings.insert(name.to_string());
                    }
                }
                let mut closure_scope = scope.enter_closure_scope(bindings);

                // Add parameters
                for param in closure.parameters.parameters.iter() {
                    let param_name = self.get_span_text(&param.variable.name.span)
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
                    }, _issues);
                }
            }
            _ => {}
        }
    }

    /// Process if statement
    fn process_if(
        &self,
        if_stmt: &IfStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        // Process condition
        self.process_expression(&if_stmt.condition, scope, issues);

        // Process body
        let mut if_scope = scope.enter_scope();
        match &if_stmt.body {
            IfStatementBody::Statement(body) => {
                match body {
                    IfStatementBodyStatement::Statement(stmt) => {
                        self.process_statement(stmt, &mut if_scope, checks, ctx, issues);
                    }
                    IfStatementBodyStatement::Block(block) => {
                        for stmt in block.statements.iter() {
                            self.process_statement(stmt, &mut if_scope, checks, ctx, issues);
                        }
                    }
                }
            }
            IfStatementBody::Block(body) => {
                for stmt in body.statements.iter() {
                    self.process_statement(stmt, &mut if_scope, checks, ctx, issues);
                }
            }
        }

        // Process elseif clauses
        for elseif in if_stmt.elseif_clauses.iter() {
            self.process_expression(&elseif.condition, scope, issues);
            let mut elseif_scope = scope.enter_scope();
            match &elseif.body {
                ElseIfClauseBody::Statement(stmt) => {
                    self.process_statement(stmt, &mut elseif_scope, checks, ctx, issues);
                }
                ElseIfClauseBody::Block(block) => {
                    for stmt in block.statements.iter() {
                        self.process_statement(stmt, &mut elseif_scope, checks, ctx, issues);
                    }
                }
            }
        }

        // Process else clause
        if let Some(else_clause) = &if_stmt.else_clause {
            let mut else_scope = scope.enter_scope();
            match &else_clause.body {
                ElseClauseBody::Statement(stmt) => {
                    self.process_statement(stmt, &mut else_scope, checks, ctx, issues);
                }
                ElseClauseBody::Block(block) => {
                    for stmt in block.statements.iter() {
                        self.process_statement(stmt, &mut else_scope, checks, ctx, issues);
                    }
                }
            }
        }
    }

    /// Process while statement
    fn process_while(
        &self,
        while_stmt: &WhileStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        self.process_expression(&while_stmt.condition, scope, issues);
        let mut loop_scope = scope.enter_scope();
        match &while_stmt.body {
            WhileStatementBody::Statement(stmt) => {
                self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
            }
            WhileStatementBody::Block(block) => {
                for stmt in block.statements.iter() {
                    self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
                }
            }
        }
    }

    /// Process do-while statement
    fn process_do_while(
        &self,
        do_while: &DoWhileStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        let mut loop_scope = scope.enter_scope();
        for stmt in do_while.body.statements.iter() {
            self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
        }
        self.process_expression(&do_while.condition, scope, issues);
    }

    /// Process for statement
    fn process_for(
        &self,
        for_stmt: &ForStatement,
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
            ForStatementBody::Statement(stmt) => {
                self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
            }
            ForStatementBody::Block(block) => {
                for stmt in block.statements.iter() {
                    self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
                }
            }
        }
    }

    /// Process foreach statement
    fn process_foreach(
        &self,
        foreach: &ForeachStatement,
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
                    let name = self.get_span_text(&direct.name.span).trim_start_matches('$');
                    loop_scope.set_variable(name.to_string(), value_type);
                }
            }
            ForeachTarget::KeyValue(kv) => {
                if let Expression::Variable(Variable::Direct(direct)) = &kv.key {
                    let name = self.get_span_text(&direct.name.span).trim_start_matches('$');
                    loop_scope.set_variable(name.to_string(), Type::Mixed);
                }
                if let Expression::Variable(Variable::Direct(direct)) = &kv.value {
                    let name = self.get_span_text(&direct.name.span).trim_start_matches('$');
                    loop_scope.set_variable(name.to_string(), value_type);
                }
            }
        }

        // Process body
        match &foreach.body {
            ForeachStatementBody::Statement(stmt) => {
                self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
            }
            ForeachStatementBody::Block(block) => {
                for stmt in block.statements.iter() {
                    self.process_statement(stmt, &mut loop_scope, checks, ctx, issues);
                }
            }
        }
    }

    /// Process switch statement
    fn process_switch(
        &self,
        switch: &SwitchStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        self.process_expression(&switch.expression, scope, issues);

        for case in switch.body.cases.iter() {
            let mut case_scope = scope.enter_scope();
            for stmt in case.statements.iter() {
                self.process_statement(stmt, &mut case_scope, checks, ctx, issues);
            }
        }

        if let Some(default) = &switch.body.default {
            let mut default_scope = scope.enter_scope();
            for stmt in default.statements.iter() {
                self.process_statement(stmt, &mut default_scope, checks, ctx, issues);
            }
        }
    }

    /// Process try statement
    fn process_try(
        &self,
        try_stmt: &TryStatement,
        scope: &mut Scope,
        checks: &[&dyn Check],
        ctx: &CheckContext<'_>,
        issues: &mut Vec<Issue>,
    ) {
        // Process try body
        let mut try_scope = scope.enter_scope();
        for stmt in try_stmt.body.statements.iter() {
            self.process_statement(stmt, &mut try_scope, checks, ctx, issues);
        }

        // Process catch clauses
        for catch in try_stmt.catch_clauses.iter() {
            let mut catch_scope = scope.enter_scope();

            // Add exception variable
            if let Some(var) = &catch.variable {
                let name = self.get_span_text(&var.name.span).trim_start_matches('$');
                let exc_type = if catch.types.is_empty() {
                    Type::object("Throwable")
                } else {
                    // Union of all caught types
                    let types: Vec<Type> = catch.types.iter()
                        .map(|t| Type::object(self.get_name_text(t)))
                        .collect();
                    if types.len() == 1 {
                        types.into_iter().next().unwrap()
                    } else {
                        Type::Union(types)
                    }
                };
                catch_scope.set_variable(name.to_string(), exc_type);
            }

            for stmt in catch.body.statements.iter() {
                self.process_statement(stmt, &mut catch_scope, checks, ctx, issues);
            }
        }

        // Process finally
        if let Some(finally) = &try_stmt.finally_clause {
            let mut finally_scope = scope.enter_scope();
            for stmt in finally.body.statements.iter() {
                self.process_statement(stmt, &mut finally_scope, checks, ctx, issues);
            }
        }
    }

    /// Get text for a span
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Get text for a Name
    fn get_name_text(&self, name: &Name) -> String {
        match name {
            Name::Resolved(resolved) => {
                self.get_span_text(&resolved.span).to_string()
            }
            Name::Unresolved(unresolved) => {
                let parts: Vec<_> = unresolved.parts.iter()
                    .map(|p| self.get_span_text(&p.span))
                    .collect();
                parts.join("\\")
            }
        }
    }
}
