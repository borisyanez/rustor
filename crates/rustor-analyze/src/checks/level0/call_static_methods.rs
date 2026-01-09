//! Check for calls to undefined static methods
//!
//! This is a placeholder for future type-aware static method call checking.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::*;

/// Checks for static method calls like Foo::bar()
///
/// This check is intentionally conservative - it only reports errors when
/// we can be certain the static method doesn't exist.
pub struct CallStaticMethodsCheck;

impl Check for CallStaticMethodsCheck {
    fn id(&self) -> &'static str {
        "staticMethod.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects calls to undefined static methods"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, _program: &Program<'a>, _ctx: &CheckContext<'_>) -> Vec<Issue> {
        // This check requires a full symbol table to be useful.
        // Without knowing all methods of a class (including inherited ones),
        // we can't determine if a static method exists.
        //
        // Future implementation will use the symbol table to provide
        // accurate static-method-not-found errors.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_method_check_id() {
        let check = CallStaticMethodsCheck;
        assert_eq!(check.id(), "staticMethod.notFound");
    }
}
