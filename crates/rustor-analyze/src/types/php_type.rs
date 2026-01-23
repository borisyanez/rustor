//! PHP type representation
//!
//! This module defines the core Type enum representing all PHP types
//! that can be tracked during static analysis.

use std::fmt;

/// Visibility modifier for class members
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Public
    }
}

/// Represents a PHP type for static analysis
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// The mixed type - accepts any value
    Mixed,

    /// The never type - function never returns (throws or exits)
    Never,

    /// The void type - function returns nothing
    Void,

    /// The null type
    Null,

    /// Boolean type (true or false)
    Bool,

    /// Constant boolean value
    ConstantBool(bool),

    /// Integer type
    Int,

    /// Constant integer value
    ConstantInt(i64),

    /// Integer range (min..max inclusive)
    IntRange {
        min: Option<i64>,
        max: Option<i64>,
    },

    /// Float type
    Float,

    /// Constant float value
    ConstantFloat(f64),

    /// String type
    String,

    /// Constant string value
    ConstantString(String),

    /// Non-empty string
    NonEmptyString,

    /// Numeric string (string that is numeric)
    NumericString,

    /// Class-string type (string containing a class name)
    ClassString {
        class_name: Option<String>,
    },

    /// Array type with key and value types
    Array {
        key: Box<Type>,
        value: Box<Type>,
    },

    /// List type (array with consecutive integer keys starting at 0)
    List {
        value: Box<Type>,
    },

    /// Non-empty array
    NonEmptyArray {
        key: Box<Type>,
        value: Box<Type>,
    },

    /// Object type (optionally of a specific class)
    Object {
        class_name: Option<String>,
    },

    /// Generic object type with type arguments (e.g., Repository<Entity>)
    GenericObject {
        class_name: String,
        type_args: Vec<Type>,
    },

    /// Callable type
    Callable,

    /// Closure type
    Closure,

    /// Resource type
    Resource,

    /// Iterable type
    Iterable {
        key: Box<Type>,
        value: Box<Type>,
    },

    /// Union of multiple types (e.g., int|string)
    Union(Vec<Type>),

    /// Intersection of multiple types (e.g., Countable&Traversable)
    Intersection(Vec<Type>),

    /// Nullable type (shorthand for T|null)
    Nullable(Box<Type>),

    /// Static type (late static binding)
    Static,

    /// Self type (current class)
    SelfType,

    /// Parent type
    Parent,

    /// Template/generic type parameter
    Template {
        name: String,
        bound: Option<Box<Type>>,
    },

    /// Literal type for specific values
    Literal(String),
}

impl Type {
    /// Create a new object type for a specific class
    pub fn object(class_name: impl Into<String>) -> Self {
        Type::Object {
            class_name: Some(class_name.into()),
        }
    }

    /// Create a new array type
    pub fn array(key: Type, value: Type) -> Self {
        Type::Array {
            key: Box::new(key),
            value: Box::new(value),
        }
    }

    /// Create a simple array with mixed key/value
    pub fn mixed_array() -> Self {
        Type::Array {
            key: Box::new(Type::Mixed),
            value: Box::new(Type::Mixed),
        }
    }

    /// Create a list type (array with int keys)
    pub fn list(value: Type) -> Self {
        Type::List {
            value: Box::new(value),
        }
    }

    /// Create a union type
    pub fn union(types: Vec<Type>) -> Self {
        if types.len() == 1 {
            return types.into_iter().next().unwrap();
        }
        Type::Union(types)
    }

    /// Create a nullable type
    pub fn nullable(inner: Type) -> Self {
        match inner {
            Type::Null => Type::Null,
            Type::Mixed => Type::Mixed,
            Type::Nullable(_) => inner,
            _ => Type::Nullable(Box::new(inner)),
        }
    }

    /// Check if this type is a scalar type
    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            Type::Bool
                | Type::ConstantBool(_)
                | Type::Int
                | Type::ConstantInt(_)
                | Type::IntRange { .. }
                | Type::Float
                | Type::ConstantFloat(_)
                | Type::String
                | Type::ConstantString(_)
                | Type::NonEmptyString
                | Type::NumericString
                | Type::ClassString { .. }
        )
    }

    /// Check if this type accepts null
    pub fn accepts_null(&self) -> bool {
        matches!(
            self,
            Type::Null | Type::Mixed | Type::Nullable(_)
        ) || matches!(self, Type::Union(types) if types.iter().any(|t| t.accepts_null()))
    }

    /// Check if this is a constant type (literal value)
    pub fn is_constant(&self) -> bool {
        matches!(
            self,
            Type::ConstantBool(_)
                | Type::ConstantInt(_)
                | Type::ConstantFloat(_)
                | Type::ConstantString(_)
                | Type::Null
        )
    }

    /// Get the generalized type (remove constant information)
    pub fn generalize(&self) -> Type {
        match self {
            Type::ConstantBool(_) => Type::Bool,
            Type::ConstantInt(_) => Type::Int,
            Type::ConstantFloat(_) => Type::Float,
            Type::ConstantString(_) => Type::String,
            Type::IntRange { .. } => Type::Int,
            Type::NonEmptyString | Type::NumericString | Type::ClassString { .. } => Type::String,
            Type::NonEmptyArray { key, value } => Type::Array {
                key: Box::new(key.generalize()),
                value: Box::new(value.generalize()),
            },
            Type::List { value } => Type::Array {
                key: Box::new(Type::Int),
                value: Box::new(value.generalize()),
            },
            Type::Array { key, value } => Type::Array {
                key: Box::new(key.generalize()),
                value: Box::new(value.generalize()),
            },
            Type::Nullable(inner) => Type::Nullable(Box::new(inner.generalize())),
            Type::Union(types) => {
                Type::Union(types.iter().map(|t| t.generalize()).collect())
            }
            Type::Intersection(types) => {
                Type::Intersection(types.iter().map(|t| t.generalize()).collect())
            }
            _ => self.clone(),
        }
    }

    /// Get the class name if this is an object type
    pub fn get_class_name(&self) -> Option<&str> {
        match self {
            Type::Object { class_name } => class_name.as_deref(),
            Type::GenericObject { class_name, .. } => Some(class_name.as_str()),
            _ => None,
        }
    }

    /// Get the type arguments if this is a generic object type
    pub fn get_type_args(&self) -> Option<&[Type]> {
        match self {
            Type::GenericObject { type_args, .. } => Some(type_args),
            _ => None,
        }
    }

    /// Check if this type is iterable
    pub fn is_iterable(&self) -> bool {
        matches!(
            self,
            Type::Array { .. }
                | Type::List { .. }
                | Type::NonEmptyArray { .. }
                | Type::Iterable { .. }
                | Type::Mixed
        ) || self.get_class_name().map_or(false, |name| {
            matches!(
                name.to_lowercase().as_str(),
                "traversable" | "iterator" | "iteratoraggregate" | "generator"
            )
        })
    }

    /// Collect all class names referenced in this type (recursively)
    pub fn collect_class_names(&self, names: &mut Vec<String>) {
        match self {
            Type::Object { class_name: Some(name) } => {
                names.push(name.clone());
            }
            Type::GenericObject { class_name, type_args } => {
                names.push(class_name.clone());
                for arg in type_args {
                    arg.collect_class_names(names);
                }
            }
            Type::ClassString { class_name: Some(name) } => {
                names.push(name.clone());
            }
            Type::Union(types) | Type::Intersection(types) => {
                for ty in types {
                    ty.collect_class_names(names);
                }
            }
            Type::Nullable(inner) => {
                inner.collect_class_names(names);
            }
            Type::Array { key, value } | Type::NonEmptyArray { key, value } => {
                key.collect_class_names(names);
                value.collect_class_names(names);
            }
            Type::List { value } => {
                value.collect_class_names(names);
            }
            Type::Iterable { key, value } => {
                key.collect_class_names(names);
                value.collect_class_names(names);
            }
            Type::Template { bound: Some(bound), .. } => {
                bound.collect_class_names(names);
            }
            _ => {}
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Mixed => write!(f, "mixed"),
            Type::Never => write!(f, "never"),
            Type::Void => write!(f, "void"),
            Type::Null => write!(f, "null"),
            Type::Bool => write!(f, "bool"),
            Type::ConstantBool(v) => write!(f, "{}", if *v { "true" } else { "false" }),
            Type::Int => write!(f, "int"),
            Type::ConstantInt(v) => write!(f, "{}", v),
            Type::IntRange { min, max } => match (min, max) {
                (Some(min), Some(max)) => write!(f, "int<{}, {}>", min, max),
                (Some(min), None) => write!(f, "int<{}, max>", min),
                (None, Some(max)) => write!(f, "int<min, {}>", max),
                (None, None) => write!(f, "int"),
            },
            Type::Float => write!(f, "float"),
            Type::ConstantFloat(v) => write!(f, "{}", v),
            Type::String => write!(f, "string"),
            Type::ConstantString(v) => write!(f, "'{}'", v.escape_default()),
            Type::NonEmptyString => write!(f, "non-empty-string"),
            Type::NumericString => write!(f, "numeric-string"),
            Type::ClassString { class_name: Some(name) } => write!(f, "class-string<{}>", name),
            Type::ClassString { class_name: None } => write!(f, "class-string"),
            Type::Array { key, value } => {
                if matches!(key.as_ref(), Type::Mixed) && matches!(value.as_ref(), Type::Mixed) {
                    write!(f, "array")
                } else {
                    write!(f, "array<{}, {}>", key, value)
                }
            }
            Type::List { value } => write!(f, "list<{}>", value),
            Type::NonEmptyArray { key, value } => write!(f, "non-empty-array<{}, {}>", key, value),
            Type::Object { class_name: Some(name) } => write!(f, "{}", name),
            Type::Object { class_name: None } => write!(f, "object"),
            Type::GenericObject { class_name, type_args } => {
                let args: Vec<_> = type_args.iter().map(|t| t.to_string()).collect();
                write!(f, "{}<{}>", class_name, args.join(", "))
            }
            Type::Callable => write!(f, "callable"),
            Type::Closure => write!(f, "Closure"),
            Type::Resource => write!(f, "resource"),
            Type::Iterable { key, value } => write!(f, "iterable<{}, {}>", key, value),
            Type::Union(types) => {
                let parts: Vec<_> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", parts.join("|"))
            }
            Type::Intersection(types) => {
                let parts: Vec<_> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", parts.join("&"))
            }
            Type::Nullable(inner) => write!(f, "?{}", inner),
            Type::Static => write!(f, "static"),
            Type::SelfType => write!(f, "self"),
            Type::Parent => write!(f, "parent"),
            Type::Template { name, bound } => {
                if let Some(bound) = bound {
                    write!(f, "{} of {}", name, bound)
                } else {
                    write!(f, "{}", name)
                }
            }
            Type::Literal(v) => write!(f, "{}", v),
        }
    }
}

impl Default for Type {
    fn default() -> Self {
        Type::Mixed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_display() {
        assert_eq!(Type::Int.to_string(), "int");
        assert_eq!(Type::String.to_string(), "string");
        assert_eq!(Type::object("Foo").to_string(), "Foo");
        assert_eq!(Type::ConstantInt(42).to_string(), "42");
        assert_eq!(Type::nullable(Type::String).to_string(), "?string");
    }

    #[test]
    fn test_union_display() {
        let union = Type::union(vec![Type::Int, Type::String]);
        assert_eq!(union.to_string(), "int|string");
    }

    #[test]
    fn test_is_scalar() {
        assert!(Type::Int.is_scalar());
        assert!(Type::String.is_scalar());
        assert!(Type::Bool.is_scalar());
        assert!(!Type::object("Foo").is_scalar());
        assert!(!Type::mixed_array().is_scalar());
    }

    #[test]
    fn test_accepts_null() {
        assert!(Type::Null.accepts_null());
        assert!(Type::Mixed.accepts_null());
        assert!(Type::nullable(Type::String).accepts_null());
        assert!(!Type::Int.accepts_null());
        assert!(!Type::String.accepts_null());
    }

    #[test]
    fn test_generalize() {
        assert_eq!(Type::ConstantInt(42).generalize(), Type::Int);
        assert_eq!(Type::ConstantString("foo".into()).generalize(), Type::String);
        assert_eq!(Type::ConstantBool(true).generalize(), Type::Bool);
    }
}
