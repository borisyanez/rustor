//! Brace and control structure fixers
//!
//! These fixers handle brace placement and control structure formatting.

mod elseif;
mod no_closing_tag;
mod switch_case_space;
mod braces_position;
mod switch_case_semicolon_to_colon;
mod declare_equal_normalize;
mod control_structure_braces;

pub use elseif::ElseifFixer;
pub use no_closing_tag::NoClosingTagFixer;
pub use switch_case_space::SwitchCaseSpaceFixer;
pub use braces_position::BracesPositionFixer;
pub use switch_case_semicolon_to_colon::SwitchCaseSemicolonToColonFixer;
pub use declare_equal_normalize::DeclareEqualNormalizeFixer;
pub use control_structure_braces::ControlStructureBracesFixer;
