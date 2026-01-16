//! Level 0 checks: Basic errors that always indicate bugs
//!
//! - Undefined functions
//! - Undefined classes
//! - Undefined static methods
//! - Undefined class constants
//! - Undefined global constants
//! - Argument count mismatches
//! - Missing return statements
//! - Invalid uses of new static()

mod undefined_function;
mod undefined_class;
mod call_static_methods;
mod class_constant;
mod undefined_constant;
mod argument_count;
mod missing_return;
mod invalid_static_new;

pub use undefined_function::UndefinedFunctionCheck;
pub use undefined_class::UndefinedClassCheck;
pub use call_static_methods::CallStaticMethodsCheck;
pub use class_constant::ClassConstantCheck;
pub use undefined_constant::UndefinedConstantCheck;
pub use argument_count::ArgumentCountCheck;
pub use missing_return::MissingReturnCheck;
pub use invalid_static_new::InvalidStaticNewCheck;
