//! Type operations for PHP type analysis
//!
//! This module provides operations for combining, comparing, and reasoning
//! about PHP types.

use super::php_type::Type;
use super::trinary_logic::TrinaryLogic;

impl Type {
    /// Check if this type is a subtype of another type
    ///
    /// Returns Yes if this type is always a subtype,
    /// Maybe if it might be, No if it definitely isn't.
    pub fn is_subtype_of(&self, other: &Type) -> TrinaryLogic {
        // Mixed accepts anything
        if matches!(other, Type::Mixed) {
            return TrinaryLogic::Yes;
        }

        // Never is subtype of everything
        if matches!(self, Type::Never) {
            return TrinaryLogic::Yes;
        }

        // Mixed is not a subtype of specific types
        if matches!(self, Type::Mixed) {
            return TrinaryLogic::Maybe;
        }

        match (self, other) {
            // Same type
            (a, b) if a == b => TrinaryLogic::Yes,

            // Null
            (Type::Null, Type::Nullable(_)) => TrinaryLogic::Yes,

            // Nullable types
            (Type::Nullable(inner), Type::Nullable(other_inner)) => {
                inner.is_subtype_of(other_inner)
            }
            (t, Type::Nullable(inner)) if !t.accepts_null() => t.is_subtype_of(inner),

            // Constant types are subtypes of their general types
            (Type::ConstantBool(_), Type::Bool) => TrinaryLogic::Yes,
            (Type::ConstantInt(_), Type::Int) => TrinaryLogic::Yes,
            (Type::IntRange { .. }, Type::Int) => TrinaryLogic::Yes,
            (Type::ConstantFloat(_), Type::Float) => TrinaryLogic::Yes,
            (Type::ConstantString(_), Type::String) => TrinaryLogic::Yes,
            (Type::NonEmptyString, Type::String) => TrinaryLogic::Yes,
            (Type::NumericString, Type::String) => TrinaryLogic::Yes,
            (Type::ClassString { .. }, Type::String) => TrinaryLogic::Yes,

            // Int is subtype of float (widening)
            (Type::Int | Type::ConstantInt(_), Type::Float) => TrinaryLogic::Yes,

            // Array subtypes
            (Type::NonEmptyArray { .. }, Type::Array { .. }) => TrinaryLogic::Yes,
            (
                Type::Array { key: k1, value: v1 },
                Type::Array { key: k2, value: v2 },
            ) => k1.is_subtype_of(k2).and(v1.is_subtype_of(v2)),

            // List is array with int keys
            (Type::List { value: v1 }, Type::Array { key: k2, value: v2 }) => {
                Type::Int.is_subtype_of(k2).and(v1.is_subtype_of(v2))
            }

            // Iterable - arrays and objects implementing Traversable
            (Type::Array { .. } | Type::List { .. } | Type::NonEmptyArray { .. }, Type::Iterable { .. }) => {
                TrinaryLogic::Yes
            }

            // Object subtyping
            (Type::Object { class_name: Some(a) }, Type::Object { class_name: Some(b) }) => {
                if a.eq_ignore_ascii_case(b) {
                    TrinaryLogic::Yes
                } else {
                    // Would need class hierarchy to determine properly
                    TrinaryLogic::Maybe
                }
            }
            (Type::Object { class_name: Some(_) }, Type::Object { class_name: None }) => {
                TrinaryLogic::Yes
            }

            // Closure is a callable
            (Type::Closure, Type::Callable) => TrinaryLogic::Yes,

            // Union types
            (Type::Union(types), other) => {
                // All members must be subtypes
                TrinaryLogic::and_all(types.iter().map(|t| t.is_subtype_of(other)))
            }
            (t, Type::Union(types)) => {
                // At least one member must accept t
                TrinaryLogic::or_all(types.iter().map(|u| t.is_subtype_of(u)))
            }

            // Intersection types
            (Type::Intersection(types), other) => {
                // At least one member must be a subtype
                TrinaryLogic::or_all(types.iter().map(|t| t.is_subtype_of(other)))
            }
            (t, Type::Intersection(types)) => {
                // Must be subtype of all members
                TrinaryLogic::and_all(types.iter().map(|u| t.is_subtype_of(u)))
            }

            // Self/static/parent - context dependent
            (Type::SelfType, _) | (_, Type::SelfType) => TrinaryLogic::Maybe,
            (Type::Static, _) | (_, Type::Static) => TrinaryLogic::Maybe,
            (Type::Parent, _) | (_, Type::Parent) => TrinaryLogic::Maybe,

            // Default: not a subtype
            _ => TrinaryLogic::No,
        }
    }

    /// Check if this type accepts a value of another type
    ///
    /// Used for parameter type checking.
    pub fn accepts(&self, other: &Type, strict_types: bool) -> TrinaryLogic {
        if matches!(self, Type::Mixed) {
            return TrinaryLogic::Yes;
        }

        // In non-strict mode, allow coercions
        if !strict_types {
            match (self, other) {
                // String accepts numeric types
                (Type::String, Type::Int | Type::Float | Type::ConstantInt(_) | Type::ConstantFloat(_)) => {
                    return TrinaryLogic::Yes;
                }
                // Int accepts float (truncation)
                (Type::Int, Type::Float | Type::ConstantFloat(_)) => {
                    return TrinaryLogic::Yes;
                }
                // Float accepts int
                (Type::Float, Type::Int | Type::ConstantInt(_)) => {
                    return TrinaryLogic::Yes;
                }
                // Bool accepts any scalar
                (Type::Bool, t) if t.is_scalar() => {
                    return TrinaryLogic::Yes;
                }
                _ => {}
            }
        }

        // Otherwise, use subtype relationship
        other.is_subtype_of(self)
    }

    /// Create a union of two types, simplifying where possible
    pub fn union_with(self, other: Type) -> Type {
        // Same type
        if self == other {
            return self;
        }

        // Mixed absorbs everything
        if matches!(self, Type::Mixed) || matches!(other, Type::Mixed) {
            return Type::Mixed;
        }

        // Never is identity for union
        if matches!(self, Type::Never) {
            return other;
        }
        if matches!(other, Type::Never) {
            return self;
        }

        // Null with non-null becomes nullable
        if matches!(self, Type::Null) && !other.accepts_null() {
            return Type::Nullable(Box::new(other));
        }
        if matches!(other, Type::Null) && !self.accepts_null() {
            return Type::Nullable(Box::new(self));
        }

        // Widen constant types
        match (&self, &other) {
            (Type::ConstantInt(_), Type::ConstantInt(_)) => return Type::Int,
            (Type::ConstantInt(_), Type::Int) | (Type::Int, Type::ConstantInt(_)) => return Type::Int,
            (Type::ConstantString(_), Type::ConstantString(_)) => return Type::String,
            (Type::ConstantString(_), Type::String) | (Type::String, Type::ConstantString(_)) => {
                return Type::String
            }
            (Type::ConstantBool(_), Type::ConstantBool(_)) => return Type::Bool,
            (Type::ConstantBool(_), Type::Bool) | (Type::Bool, Type::ConstantBool(_)) => {
                return Type::Bool
            }
            (Type::ConstantFloat(_), Type::ConstantFloat(_)) => return Type::Float,
            (Type::ConstantFloat(_), Type::Float) | (Type::Float, Type::ConstantFloat(_)) => {
                return Type::Float
            }
            _ => {}
        }

        // Int | Float = Int | Float (numeric)
        // For now, keep as union

        // Flatten unions
        let mut types = Vec::new();
        match self {
            Type::Union(inner) => types.extend(inner),
            t => types.push(t),
        }
        match other {
            Type::Union(inner) => types.extend(inner),
            t => types.push(t),
        }

        // Remove duplicates
        types.dedup();

        if types.len() == 1 {
            types.into_iter().next().unwrap()
        } else {
            Type::Union(types)
        }
    }

    /// Create an intersection of two types
    pub fn intersect_with(self, other: Type) -> Type {
        // Same type
        if self == other {
            return self;
        }

        // Mixed is identity for intersection
        if matches!(self, Type::Mixed) {
            return other;
        }
        if matches!(other, Type::Mixed) {
            return self;
        }

        // Never absorbs everything
        if matches!(self, Type::Never) || matches!(other, Type::Never) {
            return Type::Never;
        }

        // Check if one is subtype of another
        if self.is_subtype_of(&other).yes() {
            return self;
        }
        if other.is_subtype_of(&self).yes() {
            return other;
        }

        // Create intersection
        let mut types = Vec::new();
        match self {
            Type::Intersection(inner) => types.extend(inner),
            t => types.push(t),
        }
        match other {
            Type::Intersection(inner) => types.extend(inner),
            t => types.push(t),
        }

        types.dedup();

        if types.len() == 1 {
            types.into_iter().next().unwrap()
        } else {
            Type::Intersection(types)
        }
    }

    /// Remove null from a type (type narrowing after null check)
    pub fn remove_null(&self) -> Type {
        match self {
            Type::Null => Type::Never,
            Type::Nullable(inner) => inner.as_ref().clone(),
            Type::Union(types) => {
                let filtered: Vec<_> = types
                    .iter()
                    .filter(|t| !matches!(t, Type::Null))
                    .map(|t| t.remove_null())
                    .collect();
                if filtered.is_empty() {
                    Type::Never
                } else if filtered.len() == 1 {
                    filtered.into_iter().next().unwrap()
                } else {
                    Type::Union(filtered)
                }
            }
            Type::Mixed => Type::Mixed, // Can't narrow mixed
            _ => self.clone(),
        }
    }

    /// Narrow to only the null case
    pub fn keep_only_null(&self) -> Type {
        if self.accepts_null() {
            Type::Null
        } else {
            Type::Never
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_subtype_of_same() {
        assert!(Type::Int.is_subtype_of(&Type::Int).yes());
        assert!(Type::String.is_subtype_of(&Type::String).yes());
    }

    #[test]
    fn test_is_subtype_of_mixed() {
        assert!(Type::Int.is_subtype_of(&Type::Mixed).yes());
        assert!(Type::String.is_subtype_of(&Type::Mixed).yes());
        assert!(Type::object("Foo").is_subtype_of(&Type::Mixed).yes());
    }

    #[test]
    fn test_constant_subtype() {
        assert!(Type::ConstantInt(42).is_subtype_of(&Type::Int).yes());
        assert!(Type::ConstantString("foo".into()).is_subtype_of(&Type::String).yes());
    }

    #[test]
    fn test_nullable_subtype() {
        assert!(Type::Null.is_subtype_of(&Type::nullable(Type::String)).yes());
        assert!(Type::String.is_subtype_of(&Type::nullable(Type::String)).yes());
    }

    #[test]
    fn test_union_with() {
        let result = Type::Int.union_with(Type::String);
        assert!(matches!(result, Type::Union(_)));

        let result = Type::Int.union_with(Type::Int);
        assert_eq!(result, Type::Int);

        let result = Type::Null.union_with(Type::String);
        assert!(matches!(result, Type::Nullable(_)));
    }

    #[test]
    fn test_remove_null() {
        assert_eq!(Type::nullable(Type::String).remove_null(), Type::String);
        assert_eq!(Type::Null.remove_null(), Type::Never);
    }

    #[test]
    fn test_accepts_strict() {
        assert!(Type::Int.accepts(&Type::Int, true).yes());
        assert!(Type::Int.accepts(&Type::ConstantInt(42), true).yes());
        assert!(Type::Int.accepts(&Type::Float, true).no());
    }

    #[test]
    fn test_accepts_non_strict() {
        assert!(Type::Int.accepts(&Type::Float, false).yes());
        assert!(Type::String.accepts(&Type::Int, false).yes());
    }
}
