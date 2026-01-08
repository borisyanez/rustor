//! Comment fixers
//!
//! These fixers handle comment formatting and whitespace.

mod no_trailing_whitespace_in_comment;
mod single_line_comment_style;
mod multiline_whitespace_before_semicolons;
mod single_line_comment_spacing;

pub use no_trailing_whitespace_in_comment::NoTrailingWhitespaceInCommentFixer;
pub use single_line_comment_style::SingleLineCommentStyleFixer;
pub use multiline_whitespace_before_semicolons::MultilineWhitespaceBeforeSemicolonsFixer;
pub use single_line_comment_spacing::SingleLineCommentSpacingFixer;
