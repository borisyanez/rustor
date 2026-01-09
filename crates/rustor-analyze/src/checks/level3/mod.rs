//! Level 3 checks: Return types and property types
//!
//! - Return type validation
//! - Property type validation

mod property_type;
mod return_type;

pub use property_type::PropertyTypeCheck;
pub use return_type::ReturnTypeCheck;
