//! Check for access to undefined class constants
//!
//! This is a placeholder for future type-aware class constant checking.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::*;

/// Checks for class constant access like Foo::CONSTANT
///
/// This check is intentionally conservative - it only reports errors when
/// we can be certain the constant doesn't exist.
pub struct ClassConstantCheck;

impl Check for ClassConstantCheck {
    fn id(&self) -> &'static str {
        "classConstant.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects access to undefined class constants"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, _program: &Program<'a>, _ctx: &CheckContext<'_>) -> Vec<Issue> {
        // This check requires a full symbol table to be useful.
        // Without knowing all constants of a class (including inherited ones),
        // we can't determine if a class constant exists.
        //
        // Future implementation will use the symbol table to provide
        // accurate class-constant-not-found errors.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_constant_check_id() {
        let check = ClassConstantCheck;
        assert_eq!(check.id(), "classConstant.notFound");
    }
}
