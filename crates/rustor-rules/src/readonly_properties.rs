//! Rule: Add readonly modifier to properties only assigned in constructor (PHP 8.1+)
//!
//! Example:
//! ```php
//! // Before
//! class User {
//!     private string $name;
//!
//!     public function __construct(string $name) {
//!         $this->name = $name;
//!     }
//! }
//!
//! // After
//! class User {
//!     private readonly string $name;
//!
//!     public function __construct(string $name) {
//!         $this->name = $name;
//!     }
//! }
//! ```
//!
//! Requirements:
//! - Property must be typed
//! - Property must not already be readonly or static
//! - Property must be assigned in the constructor
//! - Property must NOT be assigned in any other method

use mago_span::{HasSpan, Span};
use mago_syntax::ast::*;
use rustor_core::Edit;
use std::collections::{HashMap, HashSet};

use crate::registry::{Category, PhpVersion, Rule};

/// Information about a typed property that could potentially be readonly
#[derive(Debug)]
struct PropertyInfo {
    /// Span to insert "readonly " (right after visibility modifier)
    insert_span: Span,
    /// Whether property has visibility modifier
    has_visibility: bool,
}

/// Check a parsed PHP program for properties that can be readonly
pub fn check_readonly_properties<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut edits = Vec::new();

    for stmt in program.statements.iter() {
        check_statement(stmt, source, &mut edits);
    }

    edits
}

fn check_statement<'a>(stmt: &Statement<'a>, source: &str, edits: &mut Vec<Edit>) {
    match stmt {
        Statement::Class(class) => {
            check_class(class, source, edits);
        }
        Statement::Namespace(ns) => {
            let statements = match &ns.body {
                NamespaceBody::Implicit(body) => &body.statements,
                NamespaceBody::BraceDelimited(body) => &body.statements,
            };
            for inner in statements.iter() {
                check_statement(inner, source, edits);
            }
        }
        Statement::Block(block) => {
            for inner in block.statements.iter() {
                check_statement(inner, source, edits);
            }
        }
        _ => {}
    }
}

fn check_class<'a>(class: &Class<'a>, _source: &str, edits: &mut Vec<Edit>) {
    // Step 1: Collect all typed properties that could be readonly
    let mut properties: HashMap<String, PropertyInfo> = HashMap::new();

    for member in class.members.iter() {
        if let ClassLikeMember::Property(Property::Plain(prop)) = member {
            // Must have a type hint
            if prop.hint.is_none() {
                continue;
            }

            // Check modifiers - skip if already readonly or static
            let mut has_visibility = false;
            let mut is_readonly = false;
            let mut is_static = false;
            let mut visibility_end: Option<Span> = None;

            for modifier in prop.modifiers.iter() {
                match modifier {
                    Modifier::Private(kw) | Modifier::Protected(kw) | Modifier::Public(kw) => {
                        has_visibility = true;
                        visibility_end = Some(kw.span());
                    }
                    Modifier::Readonly(_) => is_readonly = true,
                    Modifier::Static(_) => is_static = true,
                    _ => {}
                }
            }

            // Skip if already readonly or static
            if is_readonly || is_static {
                continue;
            }

            // Process each property item
            for item in prop.items.nodes.iter() {
                // Skip properties with default values - they can't be readonly
                // (readonly properties must be initialized in constructor)
                if let PropertyItem::Concrete(_) = item {
                    continue;
                }

                if let PropertyItem::Abstract(abs) = item {
                    let var_name = abs.variable.name.trim_start_matches('$').to_string();

                    // Determine where to insert "readonly "
                    let insert_span = if let Some(vis_span) = visibility_end {
                        // Insert after visibility modifier
                        Span::new(
                            vis_span.file_id,
                            vis_span.end,
                            vis_span.end,
                        )
                    } else {
                        // No visibility - insert at start of property declaration
                        // (right after attribute lists if any)
                        prop.hint.as_ref().map(|h| {
                            Span::new(h.span().file_id, h.span().start, h.span().start)
                        }).unwrap_or_else(|| prop.span())
                    };

                    properties.insert(var_name, PropertyInfo {
                        insert_span,
                        has_visibility,
                    });

                    // Only handle first property in multi-property declaration
                    break;
                }
            }
        }
    }

    if properties.is_empty() {
        return;
    }

    // Step 2: Track assignments across all methods
    let mut constructor_assignments: HashSet<String> = HashSet::new();
    let mut other_method_assignments: HashSet<String> = HashSet::new();

    for member in class.members.iter() {
        if let ClassLikeMember::Method(method) = member {
            let is_constructor = method.name.value.eq_ignore_ascii_case("__construct");

            if let MethodBody::Concrete(body) = &method.body {
                let assignments = collect_property_assignments(&body.statements);

                if is_constructor {
                    constructor_assignments.extend(assignments);
                } else {
                    other_method_assignments.extend(assignments);
                }
            }
        }
    }

    // Step 3: Find properties that are only assigned in constructor
    for (prop_name, prop_info) in properties {
        // Must be assigned in constructor
        if !constructor_assignments.contains(&prop_name) {
            continue;
        }

        // Must NOT be assigned in any other method
        if other_method_assignments.contains(&prop_name) {
            continue;
        }

        // Generate edit to add "readonly " modifier
        let replacement = if prop_info.has_visibility {
            " readonly".to_string()
        } else {
            "readonly ".to_string()
        };

        edits.push(Edit::new(
            prop_info.insert_span,
            replacement,
            format!("Add readonly to property ${}", prop_name),
        ));
    }
}

/// Collect all property names that are assigned via $this->property = ...
fn collect_property_assignments(statements: &Sequence<'_, Statement<'_>>) -> HashSet<String> {
    let mut assignments = HashSet::new();

    for stmt in statements.iter() {
        collect_assignments_from_statement(stmt, &mut assignments);
    }

    assignments
}

fn collect_assignments_from_statement(stmt: &Statement<'_>, assignments: &mut HashSet<String>) {
    match stmt {
        Statement::Expression(expr_stmt) => {
            collect_assignments_from_expression(expr_stmt.expression, assignments);
        }
        Statement::Block(block) => {
            for inner in block.statements.iter() {
                collect_assignments_from_statement(inner, assignments);
            }
        }
        Statement::If(if_stmt) => {
            collect_assignments_from_if(&if_stmt.body, assignments);
        }
        Statement::While(while_stmt) => {
            collect_assignments_from_while_body(&while_stmt.body, assignments);
        }
        Statement::DoWhile(do_while) => {
            collect_assignments_from_statement(do_while.statement, assignments);
        }
        Statement::For(for_stmt) => {
            collect_assignments_from_for_body(&for_stmt.body, assignments);
        }
        Statement::Foreach(foreach_stmt) => {
            collect_assignments_from_foreach_body(&foreach_stmt.body, assignments);
        }
        Statement::Switch(switch_stmt) => {
            collect_assignments_from_switch_body(&switch_stmt.body, assignments);
        }
        Statement::Try(try_stmt) => {
            for inner in try_stmt.block.statements.iter() {
                collect_assignments_from_statement(inner, assignments);
            }
            for catch in try_stmt.catch_clauses.iter() {
                for inner in catch.block.statements.iter() {
                    collect_assignments_from_statement(inner, assignments);
                }
            }
            if let Some(finally) = &try_stmt.finally_clause {
                for inner in finally.block.statements.iter() {
                    collect_assignments_from_statement(inner, assignments);
                }
            }
        }
        _ => {}
    }
}

fn collect_assignments_from_if(body: &IfBody<'_>, assignments: &mut HashSet<String>) {
    match body {
        IfBody::Statement(stmt_body) => {
            collect_assignments_from_statement(stmt_body.statement, assignments);
            for else_if in stmt_body.else_if_clauses.iter() {
                collect_assignments_from_statement(else_if.statement, assignments);
            }
            if let Some(else_clause) = &stmt_body.else_clause {
                collect_assignments_from_statement(else_clause.statement, assignments);
            }
        }
        IfBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                collect_assignments_from_statement(inner, assignments);
            }
            for else_if in block.else_if_clauses.iter() {
                for inner in else_if.statements.iter() {
                    collect_assignments_from_statement(inner, assignments);
                }
            }
            if let Some(else_clause) = &block.else_clause {
                for inner in else_clause.statements.iter() {
                    collect_assignments_from_statement(inner, assignments);
                }
            }
        }
    }
}

fn collect_assignments_from_while_body(body: &WhileBody<'_>, assignments: &mut HashSet<String>) {
    match body {
        WhileBody::Statement(stmt) => {
            collect_assignments_from_statement(stmt, assignments);
        }
        WhileBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                collect_assignments_from_statement(inner, assignments);
            }
        }
    }
}

fn collect_assignments_from_for_body(body: &ForBody<'_>, assignments: &mut HashSet<String>) {
    match body {
        ForBody::Statement(stmt) => {
            collect_assignments_from_statement(stmt, assignments);
        }
        ForBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                collect_assignments_from_statement(inner, assignments);
            }
        }
    }
}

fn collect_assignments_from_foreach_body(body: &ForeachBody<'_>, assignments: &mut HashSet<String>) {
    match body {
        ForeachBody::Statement(stmt) => {
            collect_assignments_from_statement(stmt, assignments);
        }
        ForeachBody::ColonDelimited(block) => {
            for inner in block.statements.iter() {
                collect_assignments_from_statement(inner, assignments);
            }
        }
    }
}

fn collect_assignments_from_switch_body(body: &SwitchBody<'_>, assignments: &mut HashSet<String>) {
    match body {
        SwitchBody::BraceDelimited(block) => {
            for case in block.cases.iter() {
                for inner in case.statements().iter() {
                    collect_assignments_from_statement(inner, assignments);
                }
            }
        }
        SwitchBody::ColonDelimited(block) => {
            for case in block.cases.iter() {
                for inner in case.statements().iter() {
                    collect_assignments_from_statement(inner, assignments);
                }
            }
        }
    }
}

fn collect_assignments_from_expression(expr: &Expression<'_>, assignments: &mut HashSet<String>) {
    if let Expression::Assignment(assign) = expr {
        // Check if LHS is $this->property
        if let Expression::Access(Access::Property(prop_access)) = &*assign.lhs {
            if let Expression::Variable(Variable::Direct(var)) = &*prop_access.object {
                let var_name = var.name.trim_start_matches('$');
                if var_name == "this" {
                    if let ClassLikeMemberSelector::Identifier(id) = &prop_access.property {
                        assignments.insert(id.value.to_string());
                    }
                }
            }
        }
    }
}

pub struct ReadonlyPropertiesRule;

impl Rule for ReadonlyPropertiesRule {
    fn name(&self) -> &'static str {
        "readonly_properties"
    }

    fn description(&self) -> &'static str {
        "Add readonly to properties only assigned in constructor"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_readonly_properties(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php81)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_readonly_properties(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Tests ====================

    #[test]
    fn test_rule_exists() {
        let rule = ReadonlyPropertiesRule;
        assert_eq!(rule.name(), "readonly_properties");
        assert_eq!(rule.min_php_version(), Some(PhpVersion::Php81));
    }

    #[test]
    fn test_simple_readonly() {
        let source = r#"<?php
class User {
    private string $name;

    public function __construct(string $name) {
        $this->name = $name;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert!(result.contains("private readonly string $name"));
    }

    #[test]
    fn test_multiple_readonly_candidates() {
        let source = r#"<?php
class User {
    private string $name;
    protected int $age;

    public function __construct(string $name, int $age) {
        $this->name = $name;
        $this->age = $age;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 2);

        let result = transform(source);
        assert!(result.contains("private readonly string $name"));
        assert!(result.contains("protected readonly int $age"));
    }

    #[test]
    fn test_public_readonly() {
        let source = r#"<?php
class Config {
    public string $value;

    public function __construct(string $value) {
        $this->value = $value;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);

        let result = transform(source);
        assert!(result.contains("public readonly string $value"));
    }

    // ==================== Skip Cases ====================

    #[test]
    fn test_skip_already_readonly() {
        let source = r#"<?php
class User {
    private readonly string $name;

    public function __construct(string $name) {
        $this->name = $name;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip already readonly properties");
    }

    #[test]
    fn test_skip_static_property() {
        let source = r#"<?php
class Counter {
    private static int $count;

    public function __construct() {
        self::$count = 0;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip static properties");
    }

    #[test]
    fn test_skip_untyped_property() {
        let source = r#"<?php
class Legacy {
    private $name;

    public function __construct($name) {
        $this->name = $name;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip untyped properties");
    }

    #[test]
    fn test_skip_property_with_default() {
        let source = r#"<?php
class Config {
    private string $env = 'production';

    public function __construct(string $env) {
        $this->env = $env;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip properties with default values");
    }

    #[test]
    fn test_skip_property_assigned_in_other_method() {
        let source = r#"<?php
class Mutable {
    private string $value;

    public function __construct(string $value) {
        $this->value = $value;
    }

    public function setValue(string $value): void {
        $this->value = $value;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip properties assigned in other methods");
    }

    #[test]
    fn test_skip_property_not_assigned_in_constructor() {
        let source = r#"<?php
class Lazy {
    private string $value;

    public function __construct() {
        // $value not assigned here
    }

    public function init(string $value): void {
        $this->value = $value;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip properties not assigned in constructor");
    }

    // ==================== Mixed Cases ====================

    #[test]
    fn test_mixed_readonly_and_mutable() {
        let source = r#"<?php
class User {
    private string $id;      // readonly candidate - only assigned in constructor
    private string $name;    // mutable - assigned in constructor AND setter

    public function __construct(string $id, string $name) {
        $this->id = $id;
        $this->name = $name;
    }

    public function setName(string $name): void {
        $this->name = $name;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1, "Should only suggest readonly for $id");

        let result = transform(source);
        assert!(result.contains("private readonly string $id"));
        assert!(result.contains("private string $name")); // unchanged
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_assignment_in_if() {
        let source = r#"<?php
class Conditional {
    private string $name;

    public function __construct(string $name) {
        $this->name = $name;
    }

    public function reset(): void {
        if (true) {
            $this->name = '';
        }
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should detect assignment in if block");
    }

    #[test]
    fn test_assignment_in_try_catch() {
        let source = r#"<?php
class TryCatch {
    private string $data;

    public function __construct() {
        $this->data = '';
    }

    public function load(): void {
        try {
            $this->data = 'loaded';
        } catch (Exception $e) {
            $this->data = 'error';
        }
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should detect assignment in try/catch");
    }

    #[test]
    fn test_namespaced_class() {
        let source = r#"<?php
namespace App\Models;

class User {
    private string $email;

    public function __construct(string $email) {
        $this->email = $email;
    }
}
"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
    }
}
