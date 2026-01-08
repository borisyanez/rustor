//! Casing fixers for PHP code
//!
//! These fixers handle keyword casing, constant casing, and other
//! case-related formatting issues.

mod lowercase_keywords;
mod constant_case;
mod lowercase_static_reference;

pub use lowercase_keywords::LowercaseKeywordsFixer;
pub use constant_case::ConstantCaseFixer;
pub use lowercase_static_reference::LowercaseStaticReferenceFixer;
