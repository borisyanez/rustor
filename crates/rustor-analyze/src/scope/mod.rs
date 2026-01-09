//! Scope tracking for static analysis
//!
//! This module provides scope management for tracking variable types,
//! class context, and function context during PHP code analysis.

pub mod scope;
pub mod class_context;
pub mod function_context;

pub use scope::Scope;
pub use class_context::{ClassContext, PropertyInfo, MethodInfo};
pub use function_context::{FunctionContext, ParameterInfo};
