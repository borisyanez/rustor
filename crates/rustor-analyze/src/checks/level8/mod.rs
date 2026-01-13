//! Level 8 checks - Nullable type strictness
//!
//! Level 8 enables:
//! - checkNullables: Report method calls and property access on nullable types

mod nullable_access;

pub use nullable_access::NullableAccessCheck;
