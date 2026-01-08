//! Miscellaneous fixers
//!
//! Fixers that don't fit into other categories.

mod cast_spaces;
mod trailing_comma_in_multiline;
mod blank_line_before_statement;
mod combine_consecutive_issets;
mod combine_consecutive_unsets;
mod explicit_string_variable;
mod heredoc_to_nowdoc;
mod list_syntax;
mod multiline_comment_opening_closing;
mod no_multiple_statements_per_line;
mod semicolon_after_instruction;
mod ternary_to_null_coalescing;
mod assign_null_coalescing_to_coalesce_equal;
mod simple_to_complex_string_variable;
mod php_unit_fqcn_annotation;
mod php_unit_method_casing;
mod php_unit_test_annotation;

pub use cast_spaces::CastSpacesFixer;
pub use trailing_comma_in_multiline::TrailingCommaInMultilineFixer;
pub use blank_line_before_statement::BlankLineBeforeStatementFixer;
pub use combine_consecutive_issets::CombineConsecutiveIssetsFixer;
pub use combine_consecutive_unsets::CombineConsecutiveUnsetsFixer;
pub use explicit_string_variable::ExplicitStringVariableFixer;
pub use heredoc_to_nowdoc::HeredocToNowdocFixer;
pub use list_syntax::ListSyntaxFixer;
pub use multiline_comment_opening_closing::MultilineCommentOpeningClosingFixer;
pub use no_multiple_statements_per_line::NoMultipleStatementsPerLineFixer;
pub use semicolon_after_instruction::SemicolonAfterInstructionFixer;
pub use ternary_to_null_coalescing::TernaryToNullCoalescingFixer;
pub use assign_null_coalescing_to_coalesce_equal::AssignNullCoalescingToCoalesceEqualFixer;
pub use simple_to_complex_string_variable::SimpleToComplexStringVariableFixer;
pub use php_unit_fqcn_annotation::PhpUnitFqcnAnnotationFixer;
pub use php_unit_method_casing::PhpUnitMethodCasingFixer;
pub use php_unit_test_annotation::PhpUnitTestAnnotationFixer;
