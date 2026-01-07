//! rustor-core: Core abstractions for PHP refactoring
//!
//! This crate provides:
//! - `Edit`: A span-based code modification
//! - `apply_edits()`: Function to apply edits preserving formatting
//! - `Visitor`: Trait for traversing PHP AST

mod edit;
pub mod visitor;

pub use edit::{apply_edits, Edit, EditError};
pub use visitor::{visit, Visitor};
