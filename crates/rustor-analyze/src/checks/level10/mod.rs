//! Level 10 checks - Implicit mixed type strictness
//!
//! Level 10 enables:
//! - checkImplicitMixed: Treat missing typehints as `mixed` and apply level 9 restrictions

mod implicit_mixed;
mod echo_non_string;

pub use implicit_mixed::ImplicitMixedCheck;
pub use echo_non_string::EchoNonStringCheck;
