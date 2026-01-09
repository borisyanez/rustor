//! Level 2 checks: Type-aware analysis
//!
//! - Undefined methods on known types
//! - Undefined properties on known types

mod call_methods;
mod property_access;

pub use call_methods::CallMethodsCheck;
pub use property_access::PropertyAccessCheck;
