//! Risky fixers that may change code behavior
//!
//! These fixers are not enabled by default because they can potentially
//! break code that relies on loose comparison or other PHP behaviors.

mod strict_comparison;
mod declare_strict_types;
mod no_alias_functions;

pub use strict_comparison::StrictComparisonFixer;
pub use declare_strict_types::DeclareStrictTypesFixer;
pub use no_alias_functions::NoAliasFunctionsFixer;
