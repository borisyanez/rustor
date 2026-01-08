//! Syntax fixers
//!
//! Fixers for PHP syntax modernization and style.

mod array_syntax;
mod single_quote;
mod yoda_style;
mod echo_tag_syntax;
mod backtick_to_shell_exec;
mod normalize_index_brace;
mod increment_style;
mod standardize_not_equals;
mod class_attributes_separation;
mod class_reference_name_casing;
mod clean_namespace;
mod declare_parentheses;
mod empty_loop_body;
mod empty_loop_condition;
mod include_fixer;
mod integer_literal_case;
mod no_alias_language_construct_call;
mod no_binary_string;
mod operator_linebreak;
mod single_space_around_construct;

pub use array_syntax::ArraySyntaxFixer;
pub use single_quote::SingleQuoteFixer;
pub use yoda_style::YodaStyleFixer;
pub use echo_tag_syntax::EchoTagSyntaxFixer;
pub use backtick_to_shell_exec::BacktickToShellExecFixer;
pub use normalize_index_brace::NormalizeIndexBraceFixer;
pub use increment_style::IncrementStyleFixer;
pub use standardize_not_equals::StandardizeNotEqualsFixer;
pub use class_attributes_separation::ClassAttributesSeparationFixer;
pub use class_reference_name_casing::ClassReferenceNameCasingFixer;
pub use clean_namespace::CleanNamespaceFixer;
pub use declare_parentheses::DeclareParenthesesFixer;
pub use empty_loop_body::EmptyLoopBodyFixer;
pub use empty_loop_condition::EmptyLoopConditionFixer;
pub use include_fixer::IncludeFixer;
pub use integer_literal_case::IntegerLiteralCaseFixer;
pub use no_alias_language_construct_call::NoAliasLanguageConstructCallFixer;
pub use no_binary_string::NoBinaryStringFixer;
pub use operator_linebreak::OperatorLinebreakFixer;
pub use single_space_around_construct::SingleSpaceAroundConstructFixer;
