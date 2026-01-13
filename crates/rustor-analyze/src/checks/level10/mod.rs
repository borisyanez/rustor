//! Level 10 checks - Implicit mixed type strictness
//!
//! Level 10 enables:
//! - checkImplicitMixed: Treat missing typehints as `mixed` and apply level 9 restrictions

mod implicit_mixed;

pub use implicit_mixed::ImplicitMixedCheck;
