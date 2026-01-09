//! Whitespace fixers for PHP code formatting
//!
//! These fixers handle line endings, indentation, trailing whitespace,
//! and other whitespace-related formatting issues.

mod trailing_whitespace;
mod line_ending;
mod indentation;
mod single_blank_line_at_eof;
mod no_whitespace_in_blank_line;
mod encoding;
mod full_opening_tag;
mod blank_line_after_opening_tag;
mod align_multiline_comment;
mod no_extra_blank_lines;
mod no_leading_namespace_whitespace;
mod no_multiline_whitespace_around_double_arrow;
mod no_singleline_whitespace_before_semicolons;
mod no_spaces_around_offset;
mod no_whitespace_before_comma_in_array;
mod space_after_semicolon;
mod trim_array_spaces;
mod type_declaration_spaces;
mod whitespace_after_comma_in_array;
mod statement_indentation;
mod array_indentation;
mod linebreak_after_opening_tag;
mod spaces_inside_parentheses;

pub use trailing_whitespace::TrailingWhitespaceFixer;
pub use line_ending::LineEndingFixer;
pub use indentation::IndentationFixer;
pub use single_blank_line_at_eof::SingleBlankLineAtEofFixer;
pub use no_whitespace_in_blank_line::NoWhitespaceInBlankLineFixer;
pub use encoding::EncodingFixer;
pub use full_opening_tag::FullOpeningTagFixer;
pub use blank_line_after_opening_tag::BlankLineAfterOpeningTagFixer;
pub use align_multiline_comment::AlignMultilineCommentFixer;
pub use no_extra_blank_lines::NoExtraBlankLinesFixer;
pub use no_leading_namespace_whitespace::NoLeadingNamespaceWhitespaceFixer;
pub use no_multiline_whitespace_around_double_arrow::NoMultilineWhitespaceAroundDoubleArrowFixer;
pub use no_singleline_whitespace_before_semicolons::NoSinglelineWhitespaceBeforeSemicolonsFixer;
pub use no_spaces_around_offset::NoSpacesAroundOffsetFixer;
pub use no_whitespace_before_comma_in_array::NoWhitespaceBeforeCommaInArrayFixer;
pub use space_after_semicolon::SpaceAfterSemicolonFixer;
pub use trim_array_spaces::TrimArraySpacesFixer;
pub use type_declaration_spaces::TypeDeclarationSpacesFixer;
pub use whitespace_after_comma_in_array::WhitespaceAfterCommaInArrayFixer;
pub use statement_indentation::StatementIndentationFixer;
pub use array_indentation::ArrayIndentationFixer;
pub use linebreak_after_opening_tag::LinebreakAfterOpeningTagFixer;
pub use spaces_inside_parentheses::SpacesInsideParenthesesFixer;
