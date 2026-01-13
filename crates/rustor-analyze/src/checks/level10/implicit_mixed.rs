//! Implicit mixed type checking (Level 10)
//!
//! When checkImplicitMixed is enabled (level 10), missing type declarations
//! are treated as implicit `mixed` and the same restrictions from level 9 apply.
//!
//! Example that fails at level 10:
//! ```php
//! function qux($value) { // No type = implicit mixed
//!     strlen($value); // ERROR: can't pass implicit mixed to string
//! }
//! ```
//!
//! TODO: Full implementation pending - this is a placeholder for level 10

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::Program;

/// Check for invalid operations on implicit mixed types
pub struct ImplicitMixedCheck;

impl Check for ImplicitMixedCheck {
    fn id(&self) -> &'static str {
        "mixed.implicitUsage"
    }

    fn description(&self) -> &'static str {
        "Checks that implicit mixed types (missing typehints) are only passed to mixed parameters"
    }

    fn level(&self) -> u8 {
        10
    }

    fn check<'a>(&self, _program: &Program<'a>, _ctx: &CheckContext<'_>) -> Vec<Issue> {
        // TODO: Implement implicit mixed checking
        // This requires:
        // 1. Identifying parameters/variables without type declarations
        // 2. Treating them as implicit `mixed`
        // 3. Applying the same restrictions as ExplicitMixedCheck
        Vec::new()
    }
}
