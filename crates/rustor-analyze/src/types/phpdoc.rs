//! PHPDoc type annotation parser
//!
//! Parses PHPDoc annotations like @param, @return, @var to extract type information.

use super::php_type::Type;

/// Parsed PHPDoc information
#[derive(Debug, Clone, Default)]
pub struct PhpDoc {
    /// Parameter types: name -> type
    pub params: Vec<(String, Type)>,
    /// Return type
    pub return_type: Option<Type>,
    /// Variable type (@var)
    pub var_type: Option<Type>,
    /// Property types (@property, @property-read, @property-write)
    pub properties: Vec<(String, Type, PropertyAccess)>,
    /// Method signatures (@method)
    pub methods: Vec<MethodSignature>,
    /// Template/generic parameters (@template)
    pub templates: Vec<String>,
    /// @throws annotations
    pub throws: Vec<Type>,
}

/// Property access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyAccess {
    ReadWrite,
    ReadOnly,
    WriteOnly,
}

/// PHPDoc method signature
#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub return_type: Type,
    pub params: Vec<(String, Type)>,
    pub is_static: bool,
}

/// Parse a PHPDoc comment block
pub fn parse_phpdoc(comment: &str) -> PhpDoc {
    let mut doc = PhpDoc::default();

    for line in comment.lines() {
        let line = line.trim()
            .trim_start_matches(['/', '*', ' '])
            .trim_end_matches(['/', '*', ' ']);

        if let Some(rest) = line.strip_prefix("@param") {
            if let Some((type_str, name)) = parse_param_line(rest.trim()) {
                if let Some(ty) = parse_type_string(&type_str) {
                    doc.params.push((name, ty));
                }
            }
        } else if let Some(rest) = line.strip_prefix("@return") {
            if let Some(ty) = parse_type_string(rest.trim()) {
                doc.return_type = Some(ty);
            }
        } else if let Some(rest) = line.strip_prefix("@var") {
            if let Some(ty) = parse_type_string(rest.trim().split_whitespace().next().unwrap_or("")) {
                doc.var_type = Some(ty);
            }
        } else if let Some(rest) = line.strip_prefix("@throws") {
            if let Some(ty) = parse_type_string(rest.trim()) {
                doc.throws.push(ty);
            }
        } else if let Some(rest) = line.strip_prefix("@template") {
            let name = rest.trim().split_whitespace().next().unwrap_or("");
            if !name.is_empty() {
                doc.templates.push(name.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("@property-read") {
            if let Some((type_str, name)) = parse_param_line(rest.trim()) {
                if let Some(ty) = parse_type_string(&type_str) {
                    doc.properties.push((name, ty, PropertyAccess::ReadOnly));
                }
            }
        } else if let Some(rest) = line.strip_prefix("@property-write") {
            if let Some((type_str, name)) = parse_param_line(rest.trim()) {
                if let Some(ty) = parse_type_string(&type_str) {
                    doc.properties.push((name, ty, PropertyAccess::WriteOnly));
                }
            }
        } else if let Some(rest) = line.strip_prefix("@property") {
            if let Some((type_str, name)) = parse_param_line(rest.trim()) {
                if let Some(ty) = parse_type_string(&type_str) {
                    doc.properties.push((name, ty, PropertyAccess::ReadWrite));
                }
            }
        }
    }

    doc
}

/// Parse a @param line: "Type $name" or "$name Type"
fn parse_param_line(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() >= 2 {
        // Check if first part is $name
        if parts[0].starts_with('$') {
            // $name Type
            let name = parts[0].trim_start_matches('$').to_string();
            let type_str = parts[1].to_string();
            Some((type_str, name))
        } else {
            // Type $name
            let type_str = parts[0].to_string();
            let name = parts[1].trim_start_matches('$').to_string();
            Some((type_str, name))
        }
    } else {
        None
    }
}

/// Parse a type string into a Type
pub fn parse_type_string(s: &str) -> Option<Type> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Handle nullable prefix
    if let Some(inner) = s.strip_prefix('?') {
        return parse_type_string(inner).map(|t| Type::Nullable(Box::new(t)));
    }

    // Handle union types (|)
    if s.contains('|') && !s.contains('<') {
        let parts: Vec<_> = s.split('|').filter_map(|p| parse_type_string(p.trim())).collect();
        if parts.is_empty() {
            return None;
        }
        if parts.len() == 1 {
            return Some(parts.into_iter().next().unwrap());
        }
        return Some(Type::Union(parts));
    }

    // Handle intersection types (&)
    if s.contains('&') && !s.contains('<') {
        let parts: Vec<_> = s.split('&').filter_map(|p| parse_type_string(p.trim())).collect();
        if parts.is_empty() {
            return None;
        }
        if parts.len() == 1 {
            return Some(parts.into_iter().next().unwrap());
        }
        return Some(Type::Intersection(parts));
    }

    // Handle array syntax: Type[] or array<Key, Value>
    if let Some(inner) = s.strip_suffix("[]") {
        let inner_type = parse_type_string(inner).unwrap_or(Type::Mixed);
        return Some(Type::List {
            value: Box::new(inner_type),
        });
    }

    // Handle generic syntax: array<K, V>, list<V>, etc.
    if let Some(start) = s.find('<') {
        if let Some(end) = s.rfind('>') {
            let base = &s[..start];
            let params = &s[start + 1..end];

            match base.to_lowercase().as_str() {
                "array" => {
                    let (key, value) = parse_generic_params(params);
                    return Some(Type::Array {
                        key: Box::new(key),
                        value: Box::new(value),
                    });
                }
                "list" => {
                    let value = parse_type_string(params.trim()).unwrap_or(Type::Mixed);
                    return Some(Type::List {
                        value: Box::new(value),
                    });
                }
                "non-empty-array" => {
                    let (key, value) = parse_generic_params(params);
                    return Some(Type::NonEmptyArray {
                        key: Box::new(key),
                        value: Box::new(value),
                    });
                }
                "iterable" => {
                    let (key, value) = parse_generic_params(params);
                    return Some(Type::Iterable {
                        key: Box::new(key),
                        value: Box::new(value),
                    });
                }
                "class-string" => {
                    let class_name = params.trim().to_string();
                    return Some(Type::ClassString {
                        class_name: if class_name.is_empty() {
                            None
                        } else {
                            Some(class_name)
                        },
                    });
                }
                "int" => {
                    // int<min, max>
                    let parts: Vec<&str> = params.split(',').collect();
                    if parts.len() == 2 {
                        let min = parts[0].trim();
                        let max = parts[1].trim();
                        return Some(Type::IntRange {
                            min: if min == "min" { None } else { min.parse().ok() },
                            max: if max == "max" { None } else { max.parse().ok() },
                        });
                    }
                }
                _ => {
                    // Generic object type - just use the base name
                    return Some(Type::Object {
                        class_name: Some(base.to_string()),
                    });
                }
            }
        }
    }

    // Simple types
    match s.to_lowercase().as_str() {
        "mixed" => Some(Type::Mixed),
        "void" => Some(Type::Void),
        "never" | "never-return" | "never-returns" | "no-return" => Some(Type::Never),
        "null" => Some(Type::Null),
        "bool" | "boolean" => Some(Type::Bool),
        "true" => Some(Type::ConstantBool(true)),
        "false" => Some(Type::ConstantBool(false)),
        "int" | "integer" => Some(Type::Int),
        "float" | "double" => Some(Type::Float),
        "string" => Some(Type::String),
        "non-empty-string" => Some(Type::NonEmptyString),
        "numeric-string" => Some(Type::NumericString),
        "class-string" => Some(Type::ClassString { class_name: None }),
        "array" => Some(Type::mixed_array()),
        "object" => Some(Type::Object { class_name: None }),
        "callable" => Some(Type::Callable),
        "closure" => Some(Type::Closure),
        "resource" => Some(Type::Resource),
        "iterable" => Some(Type::Iterable {
            key: Box::new(Type::Mixed),
            value: Box::new(Type::Mixed),
        }),
        "self" => Some(Type::SelfType),
        "static" => Some(Type::Static),
        "parent" => Some(Type::Parent),
        "$this" | "this" => Some(Type::Static),
        "scalar" => Some(Type::Union(vec![
            Type::Bool,
            Type::Int,
            Type::Float,
            Type::String,
        ])),
        "numeric" => Some(Type::Union(vec![Type::Int, Type::Float])),
        "positive-int" => Some(Type::IntRange {
            min: Some(1),
            max: None,
        }),
        "negative-int" => Some(Type::IntRange {
            min: None,
            max: Some(-1),
        }),
        "non-negative-int" => Some(Type::IntRange {
            min: Some(0),
            max: None,
        }),
        "non-positive-int" => Some(Type::IntRange {
            min: None,
            max: Some(0),
        }),
        _ => {
            // Assume it's a class name
            if s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                || s.contains('\\')
            {
                Some(Type::Object {
                    class_name: Some(s.to_string()),
                })
            } else {
                // Unknown type, treat as mixed
                None
            }
        }
    }
}

/// Parse generic parameters like "int, string" or just "string"
fn parse_generic_params(params: &str) -> (Type, Type) {
    // Simple split by comma (doesn't handle nested generics properly)
    let parts: Vec<&str> = params.splitn(2, ',').collect();

    if parts.len() == 2 {
        let key = parse_type_string(parts[0].trim()).unwrap_or(Type::Mixed);
        let value = parse_type_string(parts[1].trim()).unwrap_or(Type::Mixed);
        (key, value)
    } else {
        // Single param = value type, key is int (for list-like)
        let value = parse_type_string(params.trim()).unwrap_or(Type::Mixed);
        (Type::Int, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_types() {
        assert_eq!(parse_type_string("int"), Some(Type::Int));
        assert_eq!(parse_type_string("string"), Some(Type::String));
        assert_eq!(parse_type_string("bool"), Some(Type::Bool));
        assert_eq!(parse_type_string("null"), Some(Type::Null));
        assert_eq!(parse_type_string("void"), Some(Type::Void));
        assert_eq!(parse_type_string("mixed"), Some(Type::Mixed));
    }

    #[test]
    fn test_parse_nullable() {
        let result = parse_type_string("?string");
        assert!(matches!(result, Some(Type::Nullable(_))));
    }

    #[test]
    fn test_parse_union() {
        let result = parse_type_string("int|string");
        assert!(matches!(result, Some(Type::Union(_))));
    }

    #[test]
    fn test_parse_array_types() {
        let result = parse_type_string("string[]");
        assert!(matches!(result, Some(Type::List { .. })));

        let result = parse_type_string("array<string, int>");
        assert!(matches!(result, Some(Type::Array { .. })));
    }

    #[test]
    fn test_parse_class_name() {
        let result = parse_type_string("DateTime");
        assert!(matches!(result, Some(Type::Object { class_name: Some(_) })));
    }

    #[test]
    fn test_parse_phpdoc_param() {
        let doc = parse_phpdoc("/** @param string $name */");
        assert_eq!(doc.params.len(), 1);
        assert_eq!(doc.params[0].0, "name");
        assert_eq!(doc.params[0].1, Type::String);
    }

    #[test]
    fn test_parse_phpdoc_return() {
        let doc = parse_phpdoc("/** @return int */");
        assert_eq!(doc.return_type, Some(Type::Int));
    }

    #[test]
    fn test_parse_phpdoc_var() {
        let doc = parse_phpdoc("/** @var DateTime */");
        assert!(matches!(doc.var_type, Some(Type::Object { .. })));
    }

    #[test]
    fn test_parse_constant_types() {
        assert_eq!(parse_type_string("true"), Some(Type::ConstantBool(true)));
        assert_eq!(parse_type_string("false"), Some(Type::ConstantBool(false)));
    }

    #[test]
    fn test_parse_special_int_types() {
        assert!(matches!(
            parse_type_string("positive-int"),
            Some(Type::IntRange { min: Some(1), max: None })
        ));
    }
}
