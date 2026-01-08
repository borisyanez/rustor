//! Type hint fixers
//!
//! Fixers for PHP type hints and declarations.

mod nullable_type_declaration;
mod void_return;
mod ordered_types;
mod no_superfluous_phpdoc_tags;
mod fully_qualified_strict_types;
mod nullable_type_declaration_for_default_null_value;
mod union_type_declaration;
mod no_null_property_initialization;

pub use nullable_type_declaration::NullableTypeDeclarationFixer;
pub use void_return::VoidReturnFixer;
pub use ordered_types::OrderedTypesFixer;
pub use no_superfluous_phpdoc_tags::NoSuperfluousPhpdocTagsFixer;
pub use fully_qualified_strict_types::FullyQualifiedStrictTypesFixer;
pub use nullable_type_declaration_for_default_null_value::NullableTypeDeclarationForDefaultNullValueFixer;
pub use union_type_declaration::UnionTypeDeclarationFixer;
pub use no_null_property_initialization::NoNullPropertyInitializationFixer;
