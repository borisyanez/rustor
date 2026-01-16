//! Level 1 checks: Variable analysis and magic methods
//!
//! - Undefined variables
//! - Possibly undefined variables (control flow)
//! - Magic method warnings (__call, __get)
//! - Unused constructor parameters
//! - Redundant isset() on always-defined variables

mod magic_methods;
mod undefined_variable;
mod unused_parameter;
mod isset_variable;

pub use magic_methods::MagicMethodsCheck;
pub use undefined_variable::UndefinedVariableCheck;
pub use unused_parameter::UnusedConstructorParameterCheck;
pub use isset_variable::IssetVariableCheck;
