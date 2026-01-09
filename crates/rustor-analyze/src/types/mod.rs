//! PHP type system for static analysis
//!
//! This module provides types and operations for representing and reasoning about
//! PHP types during static analysis, following PHPStan's type system design.

pub mod php_type;
pub mod trinary_logic;
pub mod type_ops;
pub mod phpdoc;

pub use php_type::Type;
pub use trinary_logic::TrinaryLogic;
