//! Casing fixers for PHP code
//!
//! These fixers handle keyword casing, constant casing, and other
//! case-related formatting issues.

mod lowercase_keywords;
mod constant_case;
mod lowercase_static_reference;
mod native_function_casing;
mod magic_method_casing;
mod magic_constant_casing;

pub use lowercase_keywords::LowercaseKeywordsFixer;
pub use constant_case::ConstantCaseFixer;
pub use lowercase_static_reference::LowercaseStaticReferenceFixer;
pub use native_function_casing::NativeFunctionCasingFixer;
pub use magic_method_casing::MagicMethodCasingFixer;
pub use magic_constant_casing::MagicConstantCasingFixer;
