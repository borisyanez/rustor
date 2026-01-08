//! Operator spacing fixers
//!
//! These fixers handle spacing around operators.

mod concat_space;
mod binary_operator_spaces;
mod unary_operator_spaces;
mod method_chaining_indentation;
mod new_with_parentheses;
mod no_space_around_double_colon;
mod object_operator_without_whitespace;
mod ternary_operator_spaces;
mod standardize_increment;

pub use concat_space::ConcatSpaceFixer;
pub use binary_operator_spaces::BinaryOperatorSpacesFixer;
pub use unary_operator_spaces::UnaryOperatorSpacesFixer;
pub use method_chaining_indentation::MethodChainingIndentationFixer;
pub use new_with_parentheses::NewWithParenthesesFixer;
pub use no_space_around_double_colon::NoSpaceAroundDoubleColonFixer;
pub use object_operator_without_whitespace::ObjectOperatorWithoutWhitespaceFixer;
pub use ternary_operator_spaces::TernaryOperatorSpacesFixer;
pub use standardize_increment::StandardizeIncrementFixer;
