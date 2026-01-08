//! rustor-core: Core abstractions for PHP refactoring
//!
//! This crate provides:
//! - `Edit`: A span-based code modification
//! - `EditGroup`: A group of related edits for atomic application
//! - `apply_edits()`: Function to apply edits preserving formatting
//! - `apply_edit_groups()`: Function to apply edit groups atomically
//! - `Visitor`: Trait for traversing PHP AST

mod edit;
pub mod visitor;

pub use edit::{apply_edit_groups, apply_edits, Edit, EditError, EditGroup};
pub use visitor::{visit, Visitor};
