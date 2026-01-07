//! rustor-core: Core abstractions for PHP refactoring
//!
//! This crate provides:
//! - `Edit`: A span-based code modification
//! - `apply_edits()`: Function to apply edits preserving formatting

mod edit;

pub use edit::{apply_edits, Edit, EditError};
