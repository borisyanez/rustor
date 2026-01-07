//! Rule: Add readonly modifier to properties only assigned in constructor (PHP 8.1+)
//!
//! Example:
//! ```php
//! // Before
//! class User {
//!     private string $name;
//!
//!     public function __construct(string $name) {
//!         $this->name = $name;
//!     }
//! }
//!
//! // After
//! class User {
//!     private readonly string $name;
//!
//!     public function __construct(string $name) {
//!         $this->name = $name;
//!     }
//! }
//! ```
//!
//! Note: This is a complex rule requiring tracking of property assignments across
//! multiple methods. The current implementation is a framework for future expansion.

use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for properties that can be readonly
///
/// Note: This rule is currently disabled pending full implementation
/// of property assignment tracking across methods.
pub fn check_readonly_properties<'a>(_program: &Program<'a>, _source: &str) -> Vec<Edit> {
    // TODO: Implement full property assignment tracking
    // This requires:
    // 1. Collecting all typed, non-readonly, non-static properties
    // 2. Tracking all $this->property = ... assignments across all methods
    // 3. Only suggesting readonly for properties assigned exclusively in __construct
    //
    // For now, return empty to avoid false positives
    Vec::new()
}

pub struct ReadonlyPropertiesRule;

impl Rule for ReadonlyPropertiesRule {
    fn name(&self) -> &'static str {
        "readonly_properties"
    }

    fn description(&self) -> &'static str {
        "Add readonly to properties only assigned in constructor"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_readonly_properties(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php81)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_exists() {
        let rule = ReadonlyPropertiesRule;
        assert_eq!(rule.name(), "readonly_properties");
        assert_eq!(rule.min_php_version(), Some(PhpVersion::Php81));
    }

    // Note: Additional tests will be added when the full implementation is complete
}
