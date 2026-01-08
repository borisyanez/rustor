//! Level 0 checks: Basic errors that always indicate bugs
//!
//! - Undefined functions
//! - Undefined classes

mod undefined_function;
mod undefined_class;

pub use undefined_function::UndefinedFunctionCheck;
pub use undefined_class::UndefinedClassCheck;
