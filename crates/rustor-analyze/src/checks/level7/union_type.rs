//! Union type checking (Level 7)
//!
//! When checkUnionTypes is enabled (level 7+), method calls and property access
//! on union types must be valid for ALL types in the union, not just SOME.
//!
//! Example that fails at level 7:
//! ```php
//! function foo(A|B $x) {
//!     $x->methodOnlyInA(); // ERROR: B doesn't have methodOnlyInA()
//! }
//! ```
//!
//! TODO: Full implementation pending - this is a placeholder for level 7

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::Program;

/// Check for invalid method/property access on union types
pub struct UnionTypeCheck;

impl Check for UnionTypeCheck {
    fn id(&self) -> &'static str {
        "unionType.invalid"
    }

    fn description(&self) -> &'static str {
        "Checks that methods/properties accessed on union types exist on all types in the union"
    }

    fn level(&self) -> u8 {
        7
    }

    fn check<'a>(&self, _program: &Program<'a>, _ctx: &CheckContext<'_>) -> Vec<Issue> {
        // TODO: Implement union type checking
        // This requires:
        // 1. Parsing union type hints (A|B|C)
        // 2. Tracking which variables have union types
        // 3. Checking method/property access on union-typed variables
        // 4. Verifying the member exists on ALL types in the union
        Vec::new()
    }
}
