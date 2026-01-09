//! Level 0 checks: Basic errors that always indicate bugs
//!
//! - Undefined functions
//! - Undefined classes
//! - Undefined static methods
//! - Undefined class constants
//! - Argument count mismatches
//! - Missing return statements

mod undefined_function;
mod undefined_class;
mod call_static_methods;
mod class_constant;
mod argument_count;
mod missing_return;

pub use undefined_function::UndefinedFunctionCheck;
pub use undefined_class::UndefinedClassCheck;
pub use call_static_methods::CallStaticMethodsCheck;
pub use class_constant::ClassConstantCheck;
pub use argument_count::ArgumentCountCheck;
pub use missing_return::MissingReturnCheck;
