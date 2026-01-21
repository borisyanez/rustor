//! Symbol table for cross-file analysis
//!
//! This module provides a symbol table for tracking class, function, and constant
//! definitions across multiple files, enabling cross-file analysis.

pub mod symbol_table;
pub mod class_info;
pub mod function_info;

pub use symbol_table::SymbolTable;
pub use class_info::{ClassInfo, ClassKind};
pub use function_info::FunctionInfo;
