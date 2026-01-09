//! Level 0 checks: Basic errors that always indicate bugs
//!
//! - Undefined functions
//! - Undefined classes
//! - Undefined methods
//! - Undefined static methods
//! - Undefined properties
//! - Undefined class constants

mod undefined_function;
mod undefined_class;
mod call_methods;
mod call_static_methods;
mod property_access;
mod class_constant;

pub use undefined_function::UndefinedFunctionCheck;
pub use undefined_class::UndefinedClassCheck;
pub use call_methods::CallMethodsCheck;
pub use call_static_methods::CallStaticMethodsCheck;
pub use property_access::PropertyAccessCheck;
pub use class_constant::ClassConstantCheck;
