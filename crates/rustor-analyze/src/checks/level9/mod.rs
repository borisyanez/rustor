//! Level 9 checks - Explicit mixed type strictness
//!
//! Level 9 enables:
//! - checkExplicitMixed: Only allow passing explicit `mixed` to other `mixed` parameters

mod explicit_mixed;

pub use explicit_mixed::ExplicitMixedCheck;
