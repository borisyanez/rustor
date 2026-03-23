//! Level 6 checks - Missing typehints and PHPDoc type validation

mod missing_typehints;
mod already_narrowed_type;
mod phpdoc_types;
mod generics;
mod phpdoc_errors;

pub use missing_typehints::MissingTypehintCheck;
pub use already_narrowed_type::AlreadyNarrowedTypeCheck;
pub use phpdoc_types::PhpDocTypesCheck;
pub use generics::GenericsCheck;
pub use phpdoc_errors::PhpDocParseErrorCheck;
