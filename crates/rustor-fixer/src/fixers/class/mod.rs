//! Class and visibility fixers
//!
//! These fixers handle class definition formatting, visibility declarations,
//! and class structure.

mod visibility_required;
mod no_blank_lines_after_class_opening;
mod class_definition;
mod single_class_element_per_statement;
mod single_trait_insert_per_statement;
mod ordered_class_elements;

pub use visibility_required::VisibilityRequiredFixer;
pub use no_blank_lines_after_class_opening::NoBlankLinesAfterClassOpeningFixer;
pub use class_definition::ClassDefinitionFixer;
pub use single_class_element_per_statement::SingleClassElementPerStatementFixer;
pub use single_trait_insert_per_statement::SingleTraitInsertPerStatementFixer;
pub use ordered_class_elements::OrderedClassElementsFixer;
