//! Nullable type access checking (Level 8)
//!
//! When checkNullables is enabled (level 8+), accessing methods/properties
//! on nullable types without null checks is reported as an error.
//!
//! Example that fails at level 8:
//! ```php
//! function bar(?User $user) {
//!     echo $user->name; // ERROR: $user might be null
//! }
//! ```
//!
//! TODO: Full implementation pending - this is a placeholder for level 8

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::Program;

/// Check for method/property access on nullable types
pub struct NullableAccessCheck;

impl Check for NullableAccessCheck {
    fn id(&self) -> &'static str {
        "nullable.access"
    }

    fn description(&self) -> &'static str {
        "Checks for accessing methods/properties on nullable types without null checks"
    }

    fn level(&self) -> u8 {
        8
    }

    fn check<'a>(&self, _program: &Program<'a>, _ctx: &CheckContext<'_>) -> Vec<Issue> {
        // TODO: Implement nullable type checking
        // This requires:
        // 1. Tracking nullable types (?Type)
        // 2. Detecting property/method access on potentially null values
        // 3. Tracking null checks (if ($var !== null) { ... })
        // 4. Only reporting errors when null hasn't been checked
        Vec::new()
    }
}
