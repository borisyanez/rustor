//! Function declaration fixers
//!
//! These fixers handle function and method formatting.

mod function_declaration;
mod method_argument_space;
mod return_type_declaration;

pub use function_declaration::FunctionDeclarationFixer;
pub use method_argument_space::MethodArgumentSpaceFixer;
pub use return_type_declaration::ReturnTypeDeclarationFixer;
