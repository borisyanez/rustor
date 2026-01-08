//! Brace and control structure fixers
//!
//! These fixers handle brace placement and control structure formatting.

mod elseif;
mod no_closing_tag;
mod switch_case_space;
mod braces_position;

pub use elseif::ElseifFixer;
pub use no_closing_tag::NoClosingTagFixer;
pub use switch_case_space::SwitchCaseSpaceFixer;
pub use braces_position::BracesPositionFixer;
