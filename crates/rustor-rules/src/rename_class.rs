//! Rule: rename_class (Level 6 - Configurable)
//!
//! Renames class references based on a configurable mapping.
//!
//! Example configuration in .rustor.toml:
//! ```toml
//! [rules.rename_class]
//! mappings = { "OldClass" = "NewClass", "Legacy\\Service" = "Modern\\Service" }
//! ```
//!
//! Handles class references in:
//! - `new ClassName()` - instantiation
//! - Type hints: `function f(ClassName $x)` - parameter types
//! - Return types: `function f(): ClassName` - return types
//! - Property types: `private ClassName $prop;`
//! - `extends ClassName` - class inheritance
//! - `implements InterfaceName` - interface implementation
//! - `ClassName::method()` - static method calls
//! - `ClassName::$prop` - static property access
//! - `ClassName::CONST` - class constant access
//! - `catch (ClassName $e)` - exception handling
//!
//! This is a Level 6 rule because behavior is entirely determined by user config.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::Edit;
use std::collections::HashMap;

use crate::registry::{
    Category, ConfigOption, ConfigOptionType, ConfigValue, ConfigurableRule, PhpVersion, Rule,
};

/// Configuration for the rename_class rule
#[derive(Debug, Clone, Default)]
pub struct RenameClassConfig {
    /// Map of old class names to new class names
    /// Supports both simple names and fully qualified names
    pub mappings: HashMap<String, String>,
}

/// Check a parsed PHP program for class references to rename
pub fn check_rename_class<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    check_rename_class_with_config(program, source, &RenameClassConfig::default())
}

/// Check a parsed PHP program for class references to rename with configuration
pub fn check_rename_class_with_config<'a>(
    program: &Program<'a>,
    source: &str,
    config: &RenameClassConfig,
) -> Vec<Edit> {
    if config.mappings.is_empty() {
        return Vec::new();
    }

    let mut checker = RenameClassChecker {
        source,
        config,
        edits: Vec::new(),
    };
    checker.check_program(program);
    checker.edits
}

struct RenameClassChecker<'s, 'c> {
    source: &'s str,
    config: &'c RenameClassConfig,
    edits: Vec<Edit>,
}

impl<'s, 'c> RenameClassChecker<'s, 'c> {
    fn get_text(&self, span: mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Try to find a mapping for a class name (case-insensitive for class part)
    fn find_mapping(&self, class_name: &str) -> Option<&String> {
        // Try exact match first
        if let Some(new_name) = self.config.mappings.get(class_name) {
            return Some(new_name);
        }

        // Try case-insensitive match
        let class_lower = class_name.to_lowercase();
        self.config.mappings.iter().find_map(|(old, new)| {
            if old.to_lowercase() == class_lower {
                Some(new)
            } else {
                None
            }
        })
    }

    /// Check if a name should be skipped (built-in types)
    fn should_skip(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        matches!(
            lower.as_str(),
            "int" | "string" | "float" | "bool" | "array" | "object" | "callable"
                | "iterable" | "void" | "mixed" | "null" | "false" | "true" | "never"
                | "self" | "static" | "parent"
        )
    }

    fn check_program(&mut self, program: &Program<'_>) {
        for stmt in program.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::Use(use_stmt) => {
                self.check_use(use_stmt);
            }
            Statement::Class(class) => {
                self.check_class(class);
            }
            Statement::Interface(iface) => {
                self.check_interface(iface);
            }
            Statement::Trait(trait_def) => {
                self.check_trait(trait_def);
            }
            Statement::Enum(enum_def) => {
                self.check_enum(enum_def);
            }
            Statement::Function(func) => {
                self.check_function_like_params(&func.parameter_list);
                if let Some(ref ret) = func.return_type_hint {
                    self.check_hint(&ret.hint);
                }
                self.check_block(&func.body);
            }
            Statement::Namespace(ns) => {
                let statements = match &ns.body {
                    NamespaceBody::Implicit(body) => &body.statements,
                    NamespaceBody::BraceDelimited(body) => &body.statements,
                };
                for inner in statements.iter() {
                    self.check_statement(inner);
                }
            }
            Statement::Block(block) => {
                self.check_block(block);
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
                self.check_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach_stmt) => {
                self.check_foreach_body(&foreach_stmt.body);
            }
            Statement::Try(try_stmt) => {
                self.check_block(&try_stmt.block);
                for catch in try_stmt.catch_clauses.iter() {
                    // Check the exception type hint
                    self.check_hint(&catch.hint);
                    self.check_block(&catch.block);
                }
                if let Some(ref finally) = try_stmt.finally_clause {
                    self.check_block(&finally.block);
                }
            }
            Statement::Expression(expr_stmt) => {
                self.check_expression(&expr_stmt.expression);
            }
            Statement::Return(ret) => {
                if let Some(ref val) = ret.value {
                    self.check_expression(val);
                }
            }
            _ => {}
        }
    }

    fn check_class(&mut self, class: &Class<'_>) {
        // Check extends - it's an Extends struct with `types` field
        if let Some(ref extends) = class.extends {
            // Extends has a `types` field which is a sequence of identifiers
            for parent in extends.types.iter() {
                self.check_name_span(parent.span());
            }
        }

        // Check implements
        if let Some(ref implements) = class.implements {
            for iface in implements.types.iter() {
                self.check_name_span(iface.span());
            }
        }

        // Check members
        for member in class.members.iter() {
            match member {
                ClassLikeMember::Method(method) => {
                    self.check_function_like_params(&method.parameter_list);
                    if let Some(ref ret) = method.return_type_hint {
                        self.check_hint(&ret.hint);
                    }
                    if let MethodBody::Concrete(ref body) = method.body {
                        self.check_block(body);
                    }
                }
                ClassLikeMember::Property(Property::Plain(prop)) => {
                    if let Some(ref hint) = prop.hint {
                        self.check_hint(hint);
                    }
                }
                ClassLikeMember::TraitUse(trait_use) => {
                    // Check trait references in `use TraitName;`
                    for trait_name in trait_use.trait_names.iter() {
                        self.check_identifier(trait_name);
                    }
                }
                _ => {}
            }
        }
    }

    fn check_interface(&mut self, iface: &Interface<'_>) {
        // Check extends
        if let Some(ref extends) = iface.extends {
            for parent in extends.types.iter() {
                self.check_name_span(parent.span());
            }
        }

        // Check members
        for member in iface.members.iter() {
            if let ClassLikeMember::Method(method) = member {
                self.check_function_like_params(&method.parameter_list);
                if let Some(ref ret) = method.return_type_hint {
                    self.check_hint(&ret.hint);
                }
            }
        }
    }

    fn check_trait(&mut self, trait_def: &Trait<'_>) {
        // Check members
        for member in trait_def.members.iter() {
            match member {
                ClassLikeMember::Method(method) => {
                    self.check_function_like_params(&method.parameter_list);
                    if let Some(ref ret) = method.return_type_hint {
                        self.check_hint(&ret.hint);
                    }
                    if let MethodBody::Concrete(ref body) = method.body {
                        self.check_block(body);
                    }
                }
                ClassLikeMember::Property(Property::Plain(prop)) => {
                    if let Some(ref hint) = prop.hint {
                        self.check_hint(hint);
                    }
                }
                ClassLikeMember::TraitUse(trait_use) => {
                    // Check trait references in `use TraitName;`
                    for trait_name in trait_use.trait_names.iter() {
                        self.check_identifier(trait_name);
                    }
                }
                _ => {}
            }
        }
    }

    fn check_enum(&mut self, enum_def: &Enum<'_>) {
        // Check implements
        if let Some(ref implements) = enum_def.implements {
            for iface in implements.types.iter() {
                self.check_name_span(iface.span());
            }
        }

        // Check members
        for member in enum_def.members.iter() {
            match member {
                ClassLikeMember::Method(method) => {
                    self.check_function_like_params(&method.parameter_list);
                    if let Some(ref ret) = method.return_type_hint {
                        self.check_hint(&ret.hint);
                    }
                    if let MethodBody::Concrete(ref body) = method.body {
                        self.check_block(body);
                    }
                }
                ClassLikeMember::TraitUse(trait_use) => {
                    for trait_name in trait_use.trait_names.iter() {
                        self.check_identifier(trait_name);
                    }
                }
                _ => {}
            }
        }
    }

    fn check_use(&mut self, use_stmt: &Use<'_>) {
        match &use_stmt.items {
            UseItems::Sequence(seq) => {
                // use Foo, Bar;
                for item in seq.items.iter() {
                    self.check_use_item(item, None);
                }
            }
            UseItems::TypedSequence(typed_seq) => {
                // use function foo, bar; or use const FOO, BAR;
                // Only process class/interface imports, not function/const
                if typed_seq.r#type.is_function() || typed_seq.r#type.is_const() {
                    return; // Skip function and const imports
                }
                for item in typed_seq.items.iter() {
                    self.check_use_item(item, None);
                }
            }
            UseItems::TypedList(typed_list) => {
                // use function Namespace\{foo, bar};
                // Skip function and const imports
                if typed_list.r#type.is_function() || typed_list.r#type.is_const() {
                    return;
                }
                let namespace = self.get_text(typed_list.namespace.span()).to_string();
                for item in typed_list.items.iter() {
                    self.check_use_item(item, Some(&namespace));
                }
            }
            UseItems::MixedList(mixed_list) => {
                // use Namespace\{Foo, function bar, const BAZ};
                let namespace = self.get_text(mixed_list.namespace.span()).to_string();
                for maybe_typed in mixed_list.items.iter() {
                    // Skip function and const items
                    if let Some(ref use_type) = maybe_typed.r#type {
                        if use_type.is_function() || use_type.is_const() {
                            continue;
                        }
                    }
                    self.check_use_item(&maybe_typed.item, Some(&namespace));
                }
            }
        }
    }

    fn check_use_item(&mut self, item: &UseItem<'_>, namespace_prefix: Option<&str>) {
        let name_text = self.get_text(item.name.span());

        // Build the full class name for lookup
        let full_name = if let Some(ns) = namespace_prefix {
            format!("{}\\{}", ns, name_text)
        } else {
            name_text.to_string()
        };

        // Try to find a mapping
        // First try the full name, then just the class name part
        let new_name = self.find_mapping(&full_name)
            .or_else(|| {
                // Try just the last part (class name) for simple mappings
                let class_part = name_text.rsplit('\\').next().unwrap_or(name_text);
                self.find_mapping(class_part)
            });

        if let Some(new_name) = new_name {
            // If renaming, we need to replace the entire name in the use statement
            // The new name might have a different namespace structure
            self.edits.push(Edit::new(
                item.name.span(),
                new_name.clone(),
                format!("Rename use {} to {}", full_name, new_name),
            ));
        }
    }

    fn check_function_like_params(&mut self, params: &FunctionLikeParameterList<'_>) {
        for param in params.parameters.iter() {
            if let Some(ref hint) = param.hint {
                self.check_hint(hint);
            }
        }
    }

    fn check_hint(&mut self, hint: &Hint<'_>) {
        match hint {
            Hint::Identifier(ident) => {
                self.check_identifier(ident);
            }
            Hint::Nullable(nullable) => {
                self.check_hint(&nullable.hint);
            }
            Hint::Union(union) => {
                self.check_hint(&union.left);
                self.check_hint(&union.right);
            }
            Hint::Intersection(intersection) => {
                self.check_hint(&intersection.left);
                self.check_hint(&intersection.right);
            }
            Hint::Parenthesized(paren) => {
                self.check_hint(&paren.hint);
            }
            _ => {}
        }
    }

    fn check_identifier(&mut self, ident: &Identifier<'_>) {
        let class_name = self.get_text(ident.span());

        if self.should_skip(class_name) {
            return;
        }

        if let Some(new_name) = self.find_mapping(class_name) {
            self.edits.push(Edit::new(
                ident.span(),
                new_name.clone(),
                format!("Rename class {} to {}", class_name, new_name),
            ));
        }
    }

    fn check_name_span(&mut self, span: mago_span::Span) {
        let class_name = self.get_text(span);

        if self.should_skip(class_name) {
            return;
        }

        if let Some(new_name) = self.find_mapping(class_name) {
            self.edits.push(Edit::new(
                span,
                new_name.clone(),
                format!("Rename class {} to {}", class_name, new_name),
            ));
        }
    }

    fn check_block(&mut self, block: &Block<'_>) {
        for stmt in block.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_if_body(&mut self, body: &IfBody<'_>) {
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
                for inner in block.statements.iter() {
                    self.check_statement(inner);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                    for inner in else_if.statements.iter() {
                        self.check_statement(inner);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for inner in else_clause.statements.iter() {
                        self.check_statement(inner);
                    }
                }
            }
        }
    }

    fn check_while_body(&mut self, body: &WhileBody<'_>) {
        match body {
            WhileBody::Statement(stmt) => {
                self.check_statement(stmt);
            }
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner);
                }
            }
        }
    }

    fn check_for_body(&mut self, body: &ForBody<'_>) {
        match body {
            ForBody::Statement(stmt) => {
                self.check_statement(stmt);
            }
            ForBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner);
                }
            }
        }
    }

    fn check_foreach_body(&mut self, body: &ForeachBody<'_>) {
        match body {
            ForeachBody::Statement(stmt) => {
                self.check_statement(stmt);
            }
            ForeachBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner);
                }
            }
        }
    }

    fn check_expression(&mut self, expr: &Expression<'_>) {
        match expr {
            // new ClassName()
            Expression::Instantiation(inst) => {
                self.check_class_expression(&inst.class);
                for arg in inst.argument_list.iter() {
                    for arg_item in arg.arguments.iter() {
                        let arg_expr = match arg_item {
                            Argument::Positional(pos) => &pos.value,
                            Argument::Named(named) => &named.value,
                        };
                        self.check_expression(arg_expr);
                    }
                }
            }
            // ClassName::method() or ClassName::$prop or ClassName::CONST
            Expression::Call(Call::StaticMethod(call)) => {
                self.check_class_expression(&call.class);
                for arg in call.argument_list.arguments.iter() {
                    let arg_expr = match arg {
                        Argument::Positional(pos) => &pos.value,
                        Argument::Named(named) => &named.value,
                    };
                    self.check_expression(arg_expr);
                }
            }
            Expression::Access(Access::StaticProperty(access)) => {
                self.check_class_expression(&access.class);
            }
            Expression::Access(Access::ClassConstant(access)) => {
                self.check_class_expression(&access.class);
            }
            // Recursive traversal
            Expression::Call(Call::Function(call)) => {
                for arg in call.argument_list.arguments.iter() {
                    let arg_expr = match arg {
                        Argument::Positional(pos) => &pos.value,
                        Argument::Named(named) => &named.value,
                    };
                    self.check_expression(arg_expr);
                }
            }
            Expression::Call(Call::Method(call)) => {
                self.check_expression(&call.object);
                for arg in call.argument_list.arguments.iter() {
                    let arg_expr = match arg {
                        Argument::Positional(pos) => &pos.value,
                        Argument::Named(named) => &named.value,
                    };
                    self.check_expression(arg_expr);
                }
            }
            Expression::Binary(binary) => {
                self.check_expression(&binary.lhs);
                self.check_expression(&binary.rhs);
                // Check for instanceof - it's a binary expression
                if let BinaryOperator::Instanceof(_) = &binary.operator {
                    // RHS should be the class name
                    self.check_class_expression(&binary.rhs);
                }
            }
            Expression::UnaryPrefix(unary) => {
                self.check_expression(&unary.operand);
            }
            Expression::UnaryPostfix(unary) => {
                self.check_expression(&unary.operand);
            }
            Expression::Parenthesized(paren) => {
                self.check_expression(&paren.expression);
            }
            Expression::Conditional(cond) => {
                self.check_expression(&cond.condition);
                if let Some(ref then_expr) = cond.then {
                    self.check_expression(then_expr);
                }
                self.check_expression(&cond.r#else);
            }
            Expression::Assignment(assign) => {
                self.check_expression(&assign.lhs);
                self.check_expression(&assign.rhs);
            }
            Expression::Closure(closure) => {
                self.check_function_like_params(&closure.parameter_list);
                if let Some(ref ret) = closure.return_type_hint {
                    self.check_hint(&ret.hint);
                }
                self.check_block(&closure.body);
            }
            Expression::ArrowFunction(arrow) => {
                self.check_function_like_params(&arrow.parameter_list);
                if let Some(ref ret) = arrow.return_type_hint {
                    self.check_hint(&ret.hint);
                }
                self.check_expression(&arrow.expression);
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
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn check_class_expression(&mut self, class_expr: &Expression<'_>) {
        match class_expr {
            Expression::Identifier(ident) => {
                let class_name = self.get_text(ident.span());

                if self.should_skip(class_name) {
                    return;
                }

                if let Some(new_name) = self.find_mapping(class_name) {
                    self.edits.push(Edit::new(
                        ident.span(),
                        new_name.clone(),
                        format!("Rename class {} to {}", class_name, new_name),
                    ));
                }
            }
            _ => {
                // Dynamic class expression - can't rename
            }
        }
    }
}

/// Rule to rename class references based on configuration
pub struct RenameClassRule {
    config: RenameClassConfig,
}

impl RenameClassRule {
    /// Create a new rule with default (empty) configuration
    pub fn new() -> Self {
        Self {
            config: RenameClassConfig::default(),
        }
    }

    /// Create a new rule with the given mappings
    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        Self {
            config: RenameClassConfig { mappings },
        }
    }
}

impl Default for RenameClassRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Rule for RenameClassRule {
    fn name(&self) -> &'static str {
        "rename_class"
    }

    fn description(&self) -> &'static str {
        "Rename class references based on configurable mapping"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_rename_class_with_config(program, source, &self.config)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        None // No minimum version - depends on what classes are being renamed
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: &[ConfigOption] = &[ConfigOption {
            name: "mappings",
            description: "Map of old class names to new class names. Example: { \"OldClass\" = \"NewClass\" }",
            default: "{}",
            option_type: ConfigOptionType::StringMap,
        }];
        OPTIONS
    }
}

impl ConfigurableRule for RenameClassRule {
    fn with_config(config: &HashMap<String, ConfigValue>) -> Self {
        let mappings = config
            .get("mappings")
            .and_then(|v| v.as_string_map())
            .cloned()
            .unwrap_or_default();

        Self {
            config: RenameClassConfig { mappings },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php_with_config(source: &str, config: &RenameClassConfig) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_rename_class_with_config(program, source, config)
    }

    fn transform_with_config(source: &str, config: &RenameClassConfig) -> String {
        let edits = check_php_with_config(source, config);
        apply_edits(source, &edits).unwrap()
    }

    fn make_config(mappings: &[(&str, &str)]) -> RenameClassConfig {
        RenameClassConfig {
            mappings: mappings
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    // ==================== Instantiation ====================

    #[test]
    fn test_new_class() {
        let source = r#"<?php
$obj = new OldClass();
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);

        let result = transform_with_config(source, &config);
        assert!(result.contains("new NewClass()"));
    }

    #[test]
    fn test_new_class_with_args() {
        let source = r#"<?php
$obj = new OldClass($arg1, $arg2);
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("new NewClass($arg1, $arg2)"));
    }

    // ==================== Type Hints ====================

    #[test]
    fn test_parameter_type_hint() {
        let source = r#"<?php
function process(OldClass $obj) {
    return $obj;
}
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("function process(NewClass $obj)"));
    }

    #[test]
    fn test_return_type() {
        let source = r#"<?php
function create(): OldClass {
    return new OldClass();
}
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2); // return type + instantiation

        let result = transform_with_config(source, &config);
        assert!(result.contains("function create(): NewClass"));
        assert!(result.contains("new NewClass()"));
    }

    #[test]
    fn test_nullable_type_hint() {
        let source = r#"<?php
function process(?OldClass $obj): ?OldClass {
    return $obj;
}
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);

        let result = transform_with_config(source, &config);
        assert!(result.contains("?NewClass $obj"));
        assert!(result.contains("): ?NewClass"));
    }

    // ==================== Static Access ====================

    #[test]
    fn test_static_method_call() {
        let source = r#"<?php
$result = OldClass::create();
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("NewClass::create()"));
    }

    #[test]
    fn test_static_property() {
        let source = r#"<?php
$value = OldClass::$instance;
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("NewClass::$instance"));
    }

    #[test]
    fn test_class_constant() {
        let source = r#"<?php
$value = OldClass::SOME_CONST;
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("NewClass::SOME_CONST"));
    }

    // ==================== Class Definition ====================

    #[test]
    fn test_extends() {
        let source = r#"<?php
class MyClass extends OldClass {
}
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("extends NewClass"));
    }

    #[test]
    fn test_implements() {
        let source = r#"<?php
class MyClass implements OldInterface {
}
"#;
        let config = make_config(&[("OldInterface", "NewInterface")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("implements NewInterface"));
    }

    // ==================== Try/Catch ====================

    #[test]
    fn test_catch_exception() {
        let source = r#"<?php
try {
    doSomething();
} catch (OldException $e) {
    handleError($e);
}
"#;
        let config = make_config(&[("OldException", "NewException")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("catch (NewException $e)"));
    }

    // ==================== Property Types ====================

    #[test]
    fn test_property_type() {
        let source = r#"<?php
class Foo {
    private OldClass $service;
}
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let result = transform_with_config(source, &config);
        assert!(result.contains("private NewClass $service"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_self_static_parent() {
        let source = r#"<?php
class Foo extends OldClass {
    public function create(): self {
        return new static();
    }
}
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        // Should only rename extends OldClass, not self/static/parent
        assert_eq!(edits.len(), 1);
    }

    #[test]
    fn test_skip_builtin_types() {
        let source = r#"<?php
function process(int $a, string $b, array $c): bool {
    return true;
}
"#;
        let config = make_config(&[("int", "Integer"), ("string", "String")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty(), "Should not rename built-in types");
    }

    #[test]
    fn test_skip_unmatched_classes() {
        let source = r#"<?php
$obj = new SomeClass();
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_empty_config() {
        let source = r#"<?php
$obj = new OldClass();
"#;
        let config = RenameClassConfig::default();
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty());
    }

    // ==================== Complex Cases ====================

    #[test]
    fn test_closure_types() {
        let source = r#"<?php
$fn = function(OldClass $obj): OldClass {
    return $obj;
};
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_arrow_function_types() {
        let source = r#"<?php
$fn = fn(OldClass $obj): OldClass => $obj;
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Traits ====================

    #[test]
    fn test_trait_use() {
        let source = r#"<?php
class MyClass {
    use OldTrait;
}
"#;
        let config = make_config(&[("OldTrait", "NewTrait")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("use NewTrait;"));
    }

    #[test]
    fn test_trait_use_multiple() {
        let source = r#"<?php
class MyClass {
    use OldTrait, AnotherOldTrait;
}
"#;
        let config = make_config(&[("OldTrait", "NewTrait"), ("AnotherOldTrait", "AnotherNewTrait")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_trait_method_types() {
        let source = r#"<?php
trait MyTrait {
    public function process(OldClass $obj): OldClass {
        return $obj;
    }
}
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Enums ====================

    #[test]
    fn test_enum_implements() {
        let source = r#"<?php
enum Status implements OldInterface {
    case Active;
    case Inactive;
}
"#;
        let config = make_config(&[("OldInterface", "NewInterface")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("implements NewInterface"));
    }

    #[test]
    fn test_enum_method_types() {
        let source = r#"<?php
enum Status {
    case Active;

    public function process(OldClass $obj): OldClass {
        return $obj;
    }
}
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Use Statements ====================

    #[test]
    fn test_use_simple() {
        let source = r#"<?php
use OldClass;
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("use NewClass;"));
    }

    #[test]
    fn test_use_fully_qualified() {
        let source = r#"<?php
use App\Services\OldClass;
"#;
        let config = make_config(&[("App\\Services\\OldClass", "App\\Services\\NewClass")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("use App\\Services\\NewClass;"));
    }

    #[test]
    fn test_use_multiple() {
        let source = r#"<?php
use OldClass, AnotherOld;
"#;
        let config = make_config(&[("OldClass", "NewClass"), ("AnotherOld", "AnotherNew")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
        let result = transform_with_config(source, &config);
        assert!(result.contains("NewClass"));
        assert!(result.contains("AnotherNew"));
    }

    #[test]
    fn test_use_with_alias_keeps_alias() {
        let source = r#"<?php
use OldClass as Alias;
$obj = new Alias();
"#;
        let config = make_config(&[("OldClass", "NewClass")]);
        let edits = check_php_with_config(source, &config);
        // Should only rename the use statement import, not the alias usage
        assert_eq!(edits.len(), 1);
        let result = transform_with_config(source, &config);
        assert!(result.contains("use NewClass as Alias;"));
        // Alias usage should remain unchanged
        assert!(result.contains("new Alias()"));
    }

    #[test]
    fn test_use_skip_function_import() {
        let source = r#"<?php
use function oldFunc;
"#;
        let config = make_config(&[("oldFunc", "newFunc")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty(), "Should not rename function imports");
    }

    #[test]
    fn test_use_skip_const_import() {
        let source = r#"<?php
use const OLD_CONST;
"#;
        let config = make_config(&[("OLD_CONST", "NEW_CONST")]);
        let edits = check_php_with_config(source, &config);
        assert!(edits.is_empty(), "Should not rename const imports");
    }

    #[test]
    fn test_use_grouped() {
        let source = r#"<?php
use App\{OldClass, OldService};
"#;
        let config = make_config(&[("OldClass", "NewClass"), ("OldService", "NewService")]);
        let edits = check_php_with_config(source, &config);
        assert_eq!(edits.len(), 2);
    }

    // ==================== Configuration Tests ====================

    #[test]
    fn test_configurable_rule_with_config() {
        let mut config = HashMap::new();
        let mut mappings = HashMap::new();
        mappings.insert("OldClass".to_string(), "NewClass".to_string());
        config.insert("mappings".to_string(), ConfigValue::StringMap(mappings));

        let rule = RenameClassRule::with_config(&config);
        assert_eq!(rule.config.mappings.get("OldClass"), Some(&"NewClass".to_string()));
    }

    #[test]
    fn test_config_options_metadata() {
        let rule = RenameClassRule::new();
        let options = rule.config_options();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].name, "mappings");
        assert_eq!(options[0].option_type, ConfigOptionType::StringMap);
    }
}
