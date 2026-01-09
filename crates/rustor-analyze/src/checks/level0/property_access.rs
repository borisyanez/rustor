//! Check for access to undefined properties
//!
//! This is a placeholder for future type-aware property access checking.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::*;

/// Checks for property access on objects to detect undefined properties
///
/// This check is intentionally conservative - it only reports errors when
/// we can be certain the property doesn't exist.
pub struct PropertyAccessCheck;

impl Check for PropertyAccessCheck {
    fn id(&self) -> &'static str {
        "property.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects access to undefined properties"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, _program: &Program<'a>, _ctx: &CheckContext<'_>) -> Vec<Issue> {
        // This check requires type information and a symbol table to be useful.
        // Without knowing the type of $obj in $obj->property, we can't determine
        // if the property exists.
        //
        // Future implementation will use the symbol table and type inference
        // to provide accurate property-not-found errors.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_check_id() {
        let check = PropertyAccessCheck;
        assert_eq!(check.id(), "property.notFound");
    }
}
