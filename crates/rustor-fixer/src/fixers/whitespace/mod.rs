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

pub use trailing_whitespace::TrailingWhitespaceFixer;
pub use line_ending::LineEndingFixer;
pub use indentation::IndentationFixer;
pub use single_blank_line_at_eof::SingleBlankLineAtEofFixer;
pub use no_whitespace_in_blank_line::NoWhitespaceInBlankLineFixer;
pub use encoding::EncodingFixer;
pub use full_opening_tag::FullOpeningTagFixer;
pub use blank_line_after_opening_tag::BlankLineAfterOpeningTagFixer;
