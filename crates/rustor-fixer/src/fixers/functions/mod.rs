//! Function declaration fixers
//!
//! These fixers handle function and method formatting.

mod function_declaration;
mod method_argument_space;
mod return_type_declaration;
mod compact_nullable_type_declaration;
mod no_spaces_after_function_name;

pub use function_declaration::FunctionDeclarationFixer;
pub use method_argument_space::MethodArgumentSpaceFixer;
pub use return_type_declaration::ReturnTypeDeclarationFixer;
pub use compact_nullable_type_declaration::CompactNullableTypeDeclarationFixer;
pub use no_spaces_after_function_name::NoSpacesAfterFunctionNameFixer;
