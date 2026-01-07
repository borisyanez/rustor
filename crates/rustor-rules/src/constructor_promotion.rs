//! Rule: Convert constructor property assignments to promoted properties (PHP 8.0+)
//!
//! Example:
//! ```php
//! // Before
//! class User {
//!     private string $name;
//!     private int $age;
//!
//!     public function __construct(string $name, int $age) {
//!         $this->name = $name;
//!         $this->age = $age;
//!     }
//! }
//!
//! // After
//! class User {
//!     public function __construct(
//!         private string $name,
//!         private int $age,
//!     ) {}
//! }
//! ```
//!
//! Requirements for conversion:
//! - Property must be typed
//! - Property must have simple assignment in constructor ($this->prop = $param)
//! - Constructor parameter name should match property name
//! - Property should not have default value if parameter doesn't
//!
//! Note: This is a complex transformation requiring careful analysis of
//! constructor parameters, property declarations, and their relationships.
//! Full implementation would need to:
//! 1. Remove property declarations
//! 2. Add visibility modifier to constructor parameter
//! 3. Remove assignment statements from constructor body
//! This is currently a detection-only rule.

use mago_syntax::ast::*;
use rustor_core::Edit;

use crate::registry::{Category, PhpVersion, Rule};

/// Check a parsed PHP program for constructor properties that can be promoted
///
/// Note: This rule is currently a framework for detection.
/// Full implementation requires complex multi-span transformations.
pub fn check_constructor_promotion<'a>(_program: &Program<'a>, _source: &str) -> Vec<Edit> {
    // Constructor promotion transformation is complex because it requires:
    // 1. Removing property declarations (multiple spans)
    // 2. Modifying constructor parameters to add visibility
    // 3. Removing $this->prop = $param assignments from constructor body
    //
    // This requires a multi-pass approach or a different Edit model
    // that supports multiple related changes atomically.
    //
    // For now, return empty to avoid partial/broken transformations.
    Vec::new()
}

pub struct ConstructorPromotionRule;

impl Rule for ConstructorPromotionRule {
    fn name(&self) -> &'static str {
        "constructor_promotion"
    }

    fn description(&self) -> &'static str {
        "Convert constructor assignments to promoted properties"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_constructor_promotion(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_exists() {
        let rule = ConstructorPromotionRule;
        assert_eq!(rule.name(), "constructor_promotion");
        assert_eq!(rule.min_php_version(), Some(PhpVersion::Php80));
    }

    // Note: Additional tests will be added when the full implementation is complete
}
