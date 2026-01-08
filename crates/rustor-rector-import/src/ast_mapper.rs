//! AST Mapper - Maps nikic/php-parser AST types to mago-syntax types
//!
//! This module provides mappings between PHP-Parser node types (used in Rector)
//! and mago-syntax AST types (used in rustor).

use std::collections::HashMap;

/// Mapping entry from PHP-Parser to mago-syntax
#[derive(Debug, Clone)]
pub struct AstMapping {
    /// PHP-Parser type (e.g., "Expr\\FuncCall")
    pub php_type: String,

    /// mago-syntax type pattern (e.g., "Expression::Call(Call::Function(_))")
    pub mago_pattern: String,

    /// Visitor method to use (e.g., "visit_expression")
    pub visitor_method: String,

    /// Additional notes/caveats
    pub notes: Option<String>,
}

/// Build the complete mapping table
pub fn build_mapping_table() -> HashMap<String, AstMapping> {
    let mut map = HashMap::new();

    // Expression types
    add_mapping(&mut map, "Expr\\FuncCall", "Expression::Call(Call::Function(_))", "visit_expression", None);
    add_mapping(&mut map, "Expr\\MethodCall", "Expression::Call(Call::Method(_))", "visit_expression", None);
    add_mapping(&mut map, "Expr\\StaticCall", "Expression::Call(Call::StaticMethod(_))", "visit_expression", None);
    add_mapping(&mut map, "Expr\\NullsafeMethodCall", "Expression::Call(Call::NullSafeMethod(_))", "visit_expression", None);
    add_mapping(&mut map, "Expr\\New_", "Expression::Instantiation(_)", "visit_expression", None);

    // Binary operations
    add_mapping(&mut map, "Expr\\BinaryOp\\Identical", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::Identical"));
    add_mapping(&mut map, "Expr\\BinaryOp\\NotIdentical", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::NotIdentical"));
    add_mapping(&mut map, "Expr\\BinaryOp\\Equal", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::Equal"));
    add_mapping(&mut map, "Expr\\BinaryOp\\NotEqual", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::NotEqual"));
    add_mapping(&mut map, "Expr\\BinaryOp\\Concat", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::Concat"));
    add_mapping(&mut map, "Expr\\BinaryOp\\Plus", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::Addition"));
    add_mapping(&mut map, "Expr\\BinaryOp\\Minus", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::Subtraction"));
    add_mapping(&mut map, "Expr\\BinaryOp\\Mul", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::Multiplication"));
    add_mapping(&mut map, "Expr\\BinaryOp\\Div", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::Division"));
    add_mapping(&mut map, "Expr\\BinaryOp\\Pow", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::Exponentiation"));
    add_mapping(&mut map, "Expr\\BinaryOp\\Coalesce", "Expression::Binary(_)", "visit_expression", Some("Check BinaryOperator::NullCoalesce"));

    // Unary operations
    add_mapping(&mut map, "Expr\\BooleanNot", "Expression::UnaryPrefix(_)", "visit_expression", Some("Check UnaryPrefixOperator::Not"));
    add_mapping(&mut map, "Expr\\UnaryMinus", "Expression::UnaryPrefix(_)", "visit_expression", Some("Check UnaryPrefixOperator::Negation"));
    add_mapping(&mut map, "Expr\\PreInc", "Expression::UnaryPrefix(_)", "visit_expression", Some("Check UnaryPrefixOperator::Increment"));
    add_mapping(&mut map, "Expr\\PreDec", "Expression::UnaryPrefix(_)", "visit_expression", Some("Check UnaryPrefixOperator::Decrement"));
    add_mapping(&mut map, "Expr\\PostInc", "Expression::UnaryPostfix(_)", "visit_expression", Some("Check UnaryPostfixOperator::Increment"));
    add_mapping(&mut map, "Expr\\PostDec", "Expression::UnaryPostfix(_)", "visit_expression", Some("Check UnaryPostfixOperator::Decrement"));

    // Variables
    add_mapping(&mut map, "Expr\\Variable", "Expression::Variable(Variable::Direct(_))", "visit_expression", None);
    add_mapping(&mut map, "Expr\\PropertyFetch", "Expression::Access(Access::Property(_))", "visit_expression", None);
    add_mapping(&mut map, "Expr\\StaticPropertyFetch", "Expression::Access(Access::StaticProperty(_))", "visit_expression", None);
    add_mapping(&mut map, "Expr\\ArrayDimFetch", "Expression::Access(Access::ArrayAccess(_))", "visit_expression", None);

    // Literals
    add_mapping(&mut map, "Scalar\\String_", "Expression::Literal(Literal::String(_))", "visit_expression", None);
    add_mapping(&mut map, "Scalar\\LNumber", "Expression::Literal(Literal::Integer(_))", "visit_expression", None);
    add_mapping(&mut map, "Scalar\\DNumber", "Expression::Literal(Literal::Float(_))", "visit_expression", None);
    add_mapping(&mut map, "Expr\\ConstFetch", "Expression::Literal(_)", "visit_expression", Some("Check for true/false/null"));

    // Arrays
    add_mapping(&mut map, "Expr\\Array_", "Expression::Array(_)", "visit_expression", None);
    add_mapping(&mut map, "Expr\\ArrayItem", "ArrayItem", "visit_expression", Some("Part of array expression"));

    // Ternary and null coalesce
    add_mapping(&mut map, "Expr\\Ternary", "Expression::Conditional(_)", "visit_expression", None);
    add_mapping(&mut map, "Expr\\BinaryOp\\Coalesce", "Expression::Binary(_)", "visit_expression", Some("NullCoalesce operator"));

    // Closures and arrow functions
    add_mapping(&mut map, "Expr\\Closure", "Expression::Closure(_)", "visit_expression", None);
    add_mapping(&mut map, "Expr\\ArrowFunction", "Expression::ArrowFunction(_)", "visit_expression", None);

    // Casts
    add_mapping(&mut map, "Expr\\Cast\\String_", "Expression::UnaryPrefix(_)", "visit_expression", Some("CastOperator::String"));
    add_mapping(&mut map, "Expr\\Cast\\Int_", "Expression::UnaryPrefix(_)", "visit_expression", Some("CastOperator::Int"));
    add_mapping(&mut map, "Expr\\Cast\\Double", "Expression::UnaryPrefix(_)", "visit_expression", Some("CastOperator::Float"));
    add_mapping(&mut map, "Expr\\Cast\\Bool_", "Expression::UnaryPrefix(_)", "visit_expression", Some("CastOperator::Bool"));
    add_mapping(&mut map, "Expr\\Cast\\Array_", "Expression::UnaryPrefix(_)", "visit_expression", Some("CastOperator::Array"));
    add_mapping(&mut map, "Expr\\Cast\\Object_", "Expression::UnaryPrefix(_)", "visit_expression", Some("CastOperator::Object"));

    // Statements
    add_mapping(&mut map, "Stmt\\Class_", "Statement::Class(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Interface_", "Statement::Interface(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Trait_", "Statement::Trait(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Enum_", "Statement::Enum(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Function_", "Statement::Function(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\ClassMethod", "ClassLikeMember::Method(_)", "visit_class_like_member", None);
    add_mapping(&mut map, "Stmt\\Property", "ClassLikeMember::Property(_)", "visit_class_like_member", None);
    add_mapping(&mut map, "Stmt\\ClassConst", "ClassLikeMember::Constant(_)", "visit_class_like_member", None);

    // Control flow
    add_mapping(&mut map, "Stmt\\If_", "Statement::If(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Else_", "IfStatementBodyElseClause", "visit_statement", Some("Part of if statement"));
    add_mapping(&mut map, "Stmt\\ElseIf_", "IfStatementBodyElseIfClause", "visit_statement", Some("Part of if statement"));
    add_mapping(&mut map, "Stmt\\Switch_", "Statement::Switch(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Case_", "SwitchCase", "visit_statement", Some("Part of switch statement"));
    add_mapping(&mut map, "Stmt\\For_", "Statement::For(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Foreach_", "Statement::Foreach(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\While_", "Statement::While(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Do_", "Statement::DoWhile(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\TryCatch", "Statement::Try(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Catch_", "TryCatchClause", "visit_statement", Some("Part of try statement"));

    // Return/throw
    add_mapping(&mut map, "Stmt\\Return_", "Statement::Return(_)", "visit_statement", None);
    add_mapping(&mut map, "Stmt\\Throw_", "Expression::Throw(_)", "visit_expression", Some("Throw is expression in PHP 8+"));

    // Match expression (PHP 8.0+)
    add_mapping(&mut map, "Expr\\Match_", "Expression::Match(_)", "visit_expression", None);

    // Attributes (PHP 8.0+)
    add_mapping(&mut map, "Attribute", "Attribute", "visit_attribute", None);
    add_mapping(&mut map, "AttributeGroup", "AttributeList", "visit_attribute", None);

    map
}

fn add_mapping(
    map: &mut HashMap<String, AstMapping>,
    php_type: &str,
    mago_pattern: &str,
    visitor_method: &str,
    notes: Option<&str>,
) {
    map.insert(
        php_type.to_string(),
        AstMapping {
            php_type: php_type.to_string(),
            mago_pattern: mago_pattern.to_string(),
            visitor_method: visitor_method.to_string(),
            notes: notes.map(|s| s.to_string()),
        },
    );
}

/// Get the visitor method for a PHP-Parser node type
pub fn get_visitor_method(php_type: &str) -> &'static str {
    match php_type {
        // Expression types
        t if t.starts_with("Expr\\") => "visit_expression",
        t if t.starts_with("Scalar\\") => "visit_expression",

        // Statement types
        t if t.starts_with("Stmt\\") => "visit_statement",

        // Special cases
        "Attribute" | "AttributeGroup" => "visit_attribute",
        "FuncCall" => "visit_expression",
        "MethodCall" => "visit_expression",
        "StaticCall" => "visit_expression",
        "Identical" => "visit_expression",
        "NotIdentical" => "visit_expression",

        _ => "visit_expression", // Default fallback
    }
}

/// Normalize PHP-Parser type name (handle short forms)
pub fn normalize_php_type(type_name: &str) -> String {
    // Remove common prefixes/suffixes
    let normalized = type_name
        .trim()
        .replace("\\class", "")
        .replace("::class", "");

    // Handle short forms used in getNodeTypes()
    match normalized.as_str() {
        "FuncCall" => "Expr\\FuncCall".to_string(),
        "MethodCall" => "Expr\\MethodCall".to_string(),
        "StaticCall" => "Expr\\StaticCall".to_string(),
        "Identical" => "Expr\\BinaryOp\\Identical".to_string(),
        "NotIdentical" => "Expr\\BinaryOp\\NotIdentical".to_string(),
        "Array_" => "Expr\\Array_".to_string(),
        "Closure" => "Expr\\Closure".to_string(),
        "ArrowFunction" => "Expr\\ArrowFunction".to_string(),
        "Ternary" => "Expr\\Ternary".to_string(),
        "Variable" => "Expr\\Variable".to_string(),
        "PropertyFetch" => "Expr\\PropertyFetch".to_string(),
        "Class_" => "Stmt\\Class_".to_string(),
        "ClassMethod" => "Stmt\\ClassMethod".to_string(),
        "If_" => "Stmt\\If_".to_string(),
        "Switch_" => "Stmt\\Switch_".to_string(),
        "Match_" => "Expr\\Match_".to_string(),
        _ => normalized,
    }
}

/// Check if a PHP-Parser type is supported
pub fn is_type_supported(php_type: &str) -> bool {
    let mapping_table = build_mapping_table();
    let normalized = normalize_php_type(php_type);
    mapping_table.contains_key(&normalized)
}

/// Get mago-syntax pattern for a PHP-Parser type
pub fn get_mago_pattern(php_type: &str) -> Option<String> {
    let mapping_table = build_mapping_table();
    let normalized = normalize_php_type(php_type);
    mapping_table.get(&normalized).map(|m| m.mago_pattern.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_php_type() {
        assert_eq!(normalize_php_type("FuncCall"), "Expr\\FuncCall");
        assert_eq!(normalize_php_type("Identical"), "Expr\\BinaryOp\\Identical");
        assert_eq!(normalize_php_type("Closure"), "Expr\\Closure");
    }

    #[test]
    fn test_is_type_supported() {
        assert!(is_type_supported("FuncCall"));
        assert!(is_type_supported("Expr\\FuncCall"));
        assert!(is_type_supported("MethodCall"));
        assert!(is_type_supported("Identical"));
    }

    #[test]
    fn test_get_visitor_method() {
        assert_eq!(get_visitor_method("Expr\\FuncCall"), "visit_expression");
        assert_eq!(get_visitor_method("Stmt\\Class_"), "visit_statement");
        assert_eq!(get_visitor_method("FuncCall"), "visit_expression");
    }

    #[test]
    fn test_get_mago_pattern() {
        assert_eq!(
            get_mago_pattern("FuncCall"),
            Some("Expression::Call(Call::Function(_))".to_string())
        );
        assert_eq!(
            get_mago_pattern("Identical"),
            Some("Expression::Binary(_)".to_string())
        );
    }
}
