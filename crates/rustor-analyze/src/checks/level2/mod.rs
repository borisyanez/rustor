//! Level 2 checks: Type-aware analysis
//!
//! - Undefined methods on known types
//! - Undefined properties on known types

mod call_methods;
mod property_access;
mod void_pure;

pub use call_methods::CallMethodsCheck;
pub use property_access::PropertyAccessCheck;
pub use void_pure::VoidPureCheck;
