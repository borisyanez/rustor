//! Level 7 checks - Union type strictness and "maybe" reporting
//!
//! Level 7 enables:
//! - checkUnionTypes: Strict checking of methods/properties on union types
//! - reportMaybes: Report uncertain type compatibility issues

mod union_type;

pub use union_type::UnionTypeCheck;
