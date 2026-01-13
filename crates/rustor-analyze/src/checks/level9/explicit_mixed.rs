//! Explicit mixed type checking (Level 9)
//!
//! When checkExplicitMixed is enabled (level 9+), operations on explicitly
//! declared `mixed` types are restricted to only passing to other `mixed` parameters.
//!
//! Example that fails at level 9:
//! ```php
//! function baz(mixed $value) {
//!     strlen($value); // ERROR: can't pass mixed to string parameter
//!     anotherMixed($value); // OK: passing mixed to mixed
//! }
//! ```
//!
//! TODO: Full implementation pending - this is a placeholder for level 9

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::Program;

/// Check for invalid operations on explicit mixed types
pub struct ExplicitMixedCheck;

impl Check for ExplicitMixedCheck {
    fn id(&self) -> &'static str {
        "mixed.explicitUsage"
    }

    fn description(&self) -> &'static str {
        "Checks that explicit mixed types are only passed to other mixed parameters"
    }

    fn level(&self) -> u8 {
        9
    }

    fn check<'a>(&self, _program: &Program<'a>, _ctx: &CheckContext<'_>) -> Vec<Issue> {
        // TODO: Implement explicit mixed checking
        // This requires:
        // 1. Identifying parameters/variables with explicit `mixed` type
        // 2. Tracking function calls with mixed arguments
        // 3. Checking if the receiving parameter is also `mixed`
        // 4. Reporting errors when mixed is passed to non-mixed
        Vec::new()
    }
}
