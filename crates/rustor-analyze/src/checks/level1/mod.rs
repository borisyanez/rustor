//! Level 1 checks: Variable analysis and magic methods
//!
//! - Undefined variables
//! - Possibly undefined variables (control flow)
//! - Magic method warnings (__call, __get)

mod magic_methods;
mod undefined_variable;

pub use magic_methods::MagicMethodsCheck;
pub use undefined_variable::UndefinedVariableCheck;
