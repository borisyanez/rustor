//! Rule: Convert constructor property assignments to promoted properties (PHP 8.0+)
//!
//! Example:
//! ```php
//! // Before
//! class User {
//!     private string $name;
//!     private int $age;
//!
//!     public function __construct(string $name, int $age) {
//!         $this->name = $name;
//!         $this->age = $age;
//!     }
//! }
//!
//! // After
//! class User {
//!     public function __construct(
//!         private string $name,
//!         private int $age,
//!     ) {}
//! }
//! ```
//!
//! Requirements for conversion:
//! - Property must be typed
//! - Property must have simple assignment in constructor ($this->prop = $param)
//! - Constructor parameter name must match property name
//! - Property should not have default value if parameter doesn't

use mago_span::{HasSpan, Span};
use mago_syntax::ast::*;
use rustor_core::{Edit, EditGroup};
use std::collections::HashMap;

use crate::registry::{Category, PhpVersion, Rule};

/// Information about a promotable property
#[derive(Debug)]
struct PropertyInfo {
    /// The full span of the property declaration (including terminator)
    declaration_span: Span,
    /// The visibility keyword (private, protected, public)
    visibility: String,
    /// The type hint
    type_hint: String,
    /// Whether the property is readonly
    is_readonly: bool,
}

/// Information about a constructor parameter
#[derive(Debug)]
struct ParamInfo {
    /// Span of the parameter (for inserting visibility before)
    param_span: Span,
    /// Name of the parameter (without $)
    name: String,
    /// The type hint string
    type_hint: Option<String>,
    /// Whether it has a default value
    has_default: bool,
}

/// Information about a property assignment in constructor
#[derive(Debug, Clone)]
struct AssignmentInfo {
    /// Span of the full assignment statement including semicolon
    statement_span: Span,
    /// Property name being assigned to
    property_name: String,
    /// Parameter/value name being assigned from
    value_name: String,
}

/// Check a parsed PHP program for constructor properties that can be promoted
pub fn check_constructor_promotion<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
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

fn check_class<'a>(class: &Class<'a>, source: &str, edits: &mut Vec<Edit>) {
    // Skip abstract classes
    for modifier in class.modifiers.iter() {
        if let Modifier::Abstract(_) = modifier {
            return;
        }
    }

    // Step 1: Collect all typed properties without default values
    let mut properties: HashMap<String, PropertyInfo> = HashMap::new();

    for member in class.members.iter() {
        if let ClassLikeMember::Property(Property::Plain(prop)) = member {
            // Must have a type hint
            let type_hint = match &prop.hint {
                Some(hint) => extract_type_hint(hint, source),
                None => continue,
            };

            // Get visibility
            let mut visibility = String::new();
            let mut is_readonly = false;
            for modifier in prop.modifiers.iter() {
                match modifier {
                    Modifier::Private(_) => visibility = "private".to_string(),
                    Modifier::Protected(_) => visibility = "protected".to_string(),
                    Modifier::Public(_) => visibility = "public".to_string(),
                    Modifier::Readonly(_) => is_readonly = true,
                    _ => {}
                }
            }

            // Default to private if no visibility specified
            if visibility.is_empty() {
                visibility = "private".to_string();
            }

            // Process each property item (usually just one)
            for item in prop.items.nodes.iter() {
                // Skip properties with default values
                if let PropertyItem::Concrete(_) = item {
                    continue;
                }

                if let PropertyItem::Abstract(abs) = item {
                    // Get property name without $ prefix
                    let var_name = abs.variable.name.trim_start_matches('$');

                    properties.insert(
                        var_name.to_string(),
                        PropertyInfo {
                            declaration_span: prop.span(),
                            visibility,
                            type_hint,
                            is_readonly,
                        },
                    );
                    // Only handle first property in multi-property declaration
                    break;
                }
            }
        }
    }

    if properties.is_empty() {
        return;
    }

    // Step 2: Find the constructor
    let constructor = class.members.iter().find_map(|member| {
        if let ClassLikeMember::Method(method) = member {
            if method.name.value.eq_ignore_ascii_case("__construct") {
                return Some(method);
            }
        }
        None
    });

    let constructor = match constructor {
        Some(c) => c,
        None => return,
    };

    // Step 3: Collect constructor parameters
    let mut params: HashMap<String, ParamInfo> = HashMap::new();

    for param in constructor.parameter_list.parameters.nodes.iter() {
        // Skip if already promoted (has visibility modifier)
        let has_visibility = param.modifiers.iter().any(|m| {
            matches!(
                m,
                Modifier::Private(_) | Modifier::Protected(_) | Modifier::Public(_)
            )
        });
        if has_visibility {
            continue;
        }

        // Get param name without $ prefix
        let param_name = param.variable.name.trim_start_matches('$');
        let type_hint = param.hint.as_ref().map(|h| extract_type_hint(h, source));

        params.insert(
            param_name.to_string(),
            ParamInfo {
                param_span: param.span(),
                name: param_name.to_string(),
                type_hint,
                has_default: param.default_value.is_some(),
            },
        );
    }

    // Step 4: Find simple assignments in constructor body
    let assignments = match &constructor.body {
        MethodBody::Concrete(body) => collect_simple_assignments(&body.statements, source),
        _ => return,
    };

    // Step 5: Match properties to parameters via assignments
    let mut promotions: Vec<(String, PropertyInfo, ParamInfo, AssignmentInfo)> = Vec::new();

    for (prop_name, prop_info) in properties {
        // Find matching parameter
        let param_info = match params.remove(&prop_name) {
            Some(p) => p,
            None => continue,
        };

        // Find matching assignment
        let assignment = assignments
            .iter()
            .find(|a| a.property_name == prop_name && a.value_name == prop_name);

        let assignment = match assignment {
            Some(a) => a.clone(),
            None => continue,
        };

        // Type hints must match (or property has type and param doesn't)
        if let Some(param_type) = &param_info.type_hint {
            if *param_type != prop_info.type_hint {
                continue;
            }
        }

        promotions.push((prop_name, prop_info, param_info, assignment));
    }

    // Step 6: Create edits for each promotion
    for (prop_name, prop_info, param_info, assignment) in promotions {
        let mut group = EditGroup::new(
            "constructor_promotion",
            format!("Promote property ${} to constructor parameter", prop_name),
        );

        // Edit 1: Remove property declaration
        // Find the line start and include the newline at the end
        let prop_start = prop_info.declaration_span.start.offset as usize;
        let prop_end = prop_info.declaration_span.end.offset as usize;

        // Find line start
        let line_start = source[..prop_start]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);

        // Find line end including newline
        let line_end = source[prop_end..]
            .find('\n')
            .map(|i| prop_end + i + 1)
            .unwrap_or(prop_end);

        let prop_removal_span = Span::new(
            prop_info.declaration_span.file_id,
            mago_span::Position::new(line_start as u32),
            mago_span::Position::new(line_end as u32),
        );

        group.add_edit(prop_removal_span, "");

        // Edit 2: Add visibility before parameter
        let visibility_prefix = if prop_info.is_readonly {
            format!("{} readonly ", prop_info.visibility)
        } else {
            format!("{} ", prop_info.visibility)
        };

        // Insert visibility at start of parameter
        let insert_span = Span::new(
            param_info.param_span.file_id,
            param_info.param_span.start,
            param_info.param_span.start,
        );

        group.add_edit(insert_span, visibility_prefix);

        // Edit 3: Remove assignment statement
        let assign_start = assignment.statement_span.start.offset as usize;
        let assign_end = assignment.statement_span.end.offset as usize;

        // Find line start
        let assign_line_start = source[..assign_start]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);

        // Find line end including newline
        let assign_line_end = source[assign_end..]
            .find('\n')
            .map(|i| assign_end + i + 1)
            .unwrap_or(assign_end);

        let assign_removal_span = Span::new(
            assignment.statement_span.file_id,
            mago_span::Position::new(assign_line_start as u32),
            mago_span::Position::new(assign_line_end as u32),
        );

        group.add_edit(assign_removal_span, "");

        // Add all edits from the group to the output
        edits.extend(group.edits);
    }
}

/// Extract type hint as a string from the source
fn extract_type_hint(hint: &Hint<'_>, source: &str) -> String {
    let span = hint.span();
    source[span.start.offset as usize..span.end.offset as usize].to_string()
}

/// Collect simple assignments of form: $this->prop = $param;
fn collect_simple_assignments<'a>(
    statements: &Sequence<'a, Statement<'a>>,
    _source: &str,
) -> Vec<AssignmentInfo> {
    let mut assignments = Vec::new();

    for stmt in statements.iter() {
        if let Statement::Expression(expr_stmt) = stmt {
            if let Expression::Assignment(assign) = expr_stmt.expression {
                // Must be simple assignment (=), not compound (+=, etc.)
                if !matches!(assign.operator, AssignmentOperator::Assign(_)) {
                    continue;
                }

                // LHS must be $this->property
                if let Expression::Access(Access::Property(prop_access)) = &*assign.lhs {
                    // Check it's $this
                    if let Expression::Variable(Variable::Direct(var)) = &*prop_access.object {
                        // Variable name might be "$this" or "this" depending on parser
                        let var_name = var.name.trim_start_matches('$');
                        if var_name != "this" {
                            continue;
                        }
                    } else {
                        continue;
                    }

                    // Get property name
                    let prop_name = match &prop_access.property {
                        ClassLikeMemberSelector::Identifier(id) => id.value.to_string(),
                        _ => continue,
                    };

                    // RHS must be a simple variable
                    if let Expression::Variable(Variable::Direct(rhs_var)) = &*assign.rhs {
                        // Strip $ prefix for comparison
                        let value_name = rhs_var.name.trim_start_matches('$').to_string();
                        assignments.push(AssignmentInfo {
                            statement_span: stmt.span(),
                            property_name: prop_name,
                            value_name,
                        });
                    }
                }
            }
        }
    }

    assignments
}

pub struct ConstructorPromotionRule;

impl Rule for ConstructorPromotionRule {
    fn name(&self) -> &'static str {
        "constructor_promotion"
    }

    fn description(&self) -> &'static str {
        "Convert constructor assignments to promoted properties"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_constructor_promotion(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
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
        check_constructor_promotion(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    // ==================== Basic Tests ====================

    #[test]
    fn test_rule_exists() {
        let rule = ConstructorPromotionRule;
        assert_eq!(rule.name(), "constructor_promotion");
        assert_eq!(rule.min_php_version(), Some(PhpVersion::Php80));
    }

    #[test]
    fn test_simple_promotion() {
        let source = r#"<?php
class User {
    private string $name;

    public function __construct(string $name) {
        $this->name = $name;
    }
}
"#;
        let edits = check_php(source);
        assert!(!edits.is_empty(), "Should find promotion opportunity");

        let result = transform(source);
        assert!(
            result.contains("private string $name"),
            "Should have promoted parameter"
        );
    }

    #[test]
    fn test_multiple_properties() {
        let source = r#"<?php
class User {
    private string $name;
    private int $age;

    public function __construct(string $name, int $age) {
        $this->name = $name;
        $this->age = $age;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.len() >= 4, "Should have edits for both properties");

        let result = transform(source);
        assert!(result.contains("private string $name"));
        assert!(result.contains("private int $age"));
    }

    #[test]
    fn test_readonly_property() {
        let source = r#"<?php
class User {
    private readonly string $id;

    public function __construct(string $id) {
        $this->id = $id;
    }
}
"#;
        let edits = check_php(source);
        assert!(!edits.is_empty());

        let result = transform(source);
        assert!(result.contains("private readonly string $id"));
    }

    #[test]
    fn test_protected_visibility() {
        let source = r#"<?php
class Base {
    protected string $name;

    public function __construct(string $name) {
        $this->name = $name;
    }
}
"#;
        let edits = check_php(source);
        assert!(!edits.is_empty());

        let result = transform(source);
        assert!(result.contains("protected string $name"));
    }

    // ==================== Skip Cases ====================

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
        assert!(
            edits.is_empty(),
            "Should skip properties with default values"
        );
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
    fn test_skip_no_constructor() {
        let source = r#"<?php
class Data {
    private string $value;
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip classes without constructor");
    }

    #[test]
    fn test_skip_already_promoted() {
        let source = r#"<?php
class User {
    public function __construct(
        private string $name,
    ) {}
}
"#;
        let edits = check_php(source);
        assert!(
            edits.is_empty(),
            "Should skip already promoted parameters"
        );
    }

    #[test]
    fn test_skip_no_matching_assignment() {
        let source = r#"<?php
class User {
    private string $name;

    public function __construct(string $name) {
        // No assignment here
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip if no matching assignment");
    }

    #[test]
    fn test_skip_different_param_name() {
        let source = r#"<?php
class User {
    private string $name;

    public function __construct(string $username) {
        $this->name = $username;
    }
}
"#;
        let edits = check_php(source);
        assert!(
            edits.is_empty(),
            "Should skip if param name doesn't match property"
        );
    }

    #[test]
    fn test_skip_compound_assignment() {
        let source = r#"<?php
class Counter {
    private int $count;

    public function __construct(int $count) {
        $this->count += $count;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip compound assignments");
    }

    #[test]
    fn test_skip_abstract_class() {
        let source = r#"<?php
abstract class Base {
    private string $name;

    public function __construct(string $name) {
        $this->name = $name;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip abstract classes");
    }

    // ==================== Edge Cases ====================

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
        assert!(!edits.is_empty(), "Should work in namespaced classes");
    }

    #[test]
    fn test_type_mismatch_skipped() {
        let source = r#"<?php
class User {
    private string $name;

    public function __construct(int $name) {
        $this->name = $name;
    }
}
"#;
        let edits = check_php(source);
        assert!(edits.is_empty(), "Should skip if types don't match");
    }
}
