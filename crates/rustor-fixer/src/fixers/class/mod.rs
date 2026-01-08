//! Class and visibility fixers
//!
//! These fixers handle class definition formatting, visibility declarations,
//! and class structure.

mod visibility_required;
mod no_blank_lines_after_class_opening;
mod class_definition;

pub use visibility_required::VisibilityRequiredFixer;
pub use no_blank_lines_after_class_opening::NoBlankLinesAfterClassOpeningFixer;
pub use class_definition::ClassDefinitionFixer;
