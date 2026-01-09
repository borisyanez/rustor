//! Symbol collection and type resolution for static analysis
//!
//! This module provides utilities for building symbol tables from PHP code.

pub mod symbol_collector;

pub use symbol_collector::{SymbolCollector, CollectedSymbols};
