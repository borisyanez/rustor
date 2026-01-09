//! Check for method calls on objects
//!
//! This is a placeholder for future type-aware method call checking.
//! Currently, we don't flag method calls as errors since we don't have
//! full type information for variables.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::*;

/// Checks for method calls on objects
///
/// This check is intentionally conservative - it only reports errors when
/// we can be certain about the type of the object being called.
pub struct CallMethodsCheck;

impl Check for CallMethodsCheck {
    fn id(&self) -> &'static str {
        "method.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects method calls that are likely undefined"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, _program: &Program<'a>, _ctx: &CheckContext<'_>) -> Vec<Issue> {
        // This check requires type information to be useful.
        // Without knowing the type of $obj in $obj->method(), we can't determine
        // if the method exists.
        //
        // Future implementation will use the symbol table and type inference
        // to provide accurate method-not-found errors.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_check_id() {
        let check = CallMethodsCheck;
        assert_eq!(check.id(), "method.notFound");
    }
}
