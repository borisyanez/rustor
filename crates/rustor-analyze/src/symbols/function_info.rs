//! Function information for symbol table
//!
//! Stores metadata about functions for cross-file analysis.

use crate::types::Type;
use std::path::PathBuf;

/// Information about a function stored in the symbol table
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    /// Short name (without namespace)
    pub name: String,
    /// Fully qualified name
    pub full_name: String,
    /// Namespace (if any)
    pub namespace: Option<String>,
    /// Parameters
    pub parameters: Vec<FunctionParameterInfo>,
    /// Return type
    pub return_type: Option<Type>,
    /// Whether it returns by reference
    pub returns_reference: bool,
    /// File where this function is defined
    pub file: Option<PathBuf>,
    /// Line number where defined
    pub line: Option<usize>,
}

impl FunctionInfo {
    /// Create a new function info
    pub fn new(name: impl Into<String>, full_name: impl Into<String>) -> Self {
        let name = name.into();
        let full_name = full_name.into();
        let namespace = if full_name.contains('\\') {
            Some(full_name.rsplit_once('\\').unwrap().0.to_string())
        } else {
            None
        };

        Self {
            name,
            full_name,
            namespace,
            parameters: Vec::new(),
            return_type: None,
            returns_reference: false,
            file: None,
            line: None,
        }
    }

    /// Create from a fully qualified name
    pub fn from_fqn(fqn: impl Into<String>) -> Self {
        let fqn = fqn.into();
        let name = fqn.rsplit('\\').next().unwrap_or(&fqn).to_string();
        Self::new(name, fqn)
    }

    /// Add a parameter
    pub fn with_parameter(mut self, param: FunctionParameterInfo) -> Self {
        self.parameters.push(param);
        self
    }

    /// Set the return type
    pub fn with_return_type(mut self, ty: Type) -> Self {
        self.return_type = Some(ty);
        self
    }

    /// Get the minimum required argument count
    pub fn required_args(&self) -> usize {
        self.parameters
            .iter()
            .take_while(|p| !p.is_optional && !p.is_variadic)
            .count()
    }

    /// Get the maximum argument count (None if variadic)
    pub fn max_args(&self) -> Option<usize> {
        if self.parameters.iter().any(|p| p.is_variadic) {
            None
        } else {
            Some(self.parameters.len())
        }
    }

    /// Check if the function accepts a given argument count
    pub fn accepts_arg_count(&self, count: usize) -> bool {
        let min = self.required_args();
        let max = self.max_args();
        count >= min && max.map_or(true, |m| count <= m)
    }

    /// Get parameter by index
    pub fn get_parameter(&self, index: usize) -> Option<&FunctionParameterInfo> {
        self.parameters.get(index)
    }

    /// Get parameter by name
    pub fn get_parameter_by_name(&self, name: &str) -> Option<&FunctionParameterInfo> {
        self.parameters.iter().find(|p| p.name == name)
    }
}

/// Information about a function parameter
#[derive(Debug, Clone)]
pub struct FunctionParameterInfo {
    /// Parameter name (without $)
    pub name: String,
    /// Parameter type
    pub type_: Option<Type>,
    /// Whether it has a default value
    pub is_optional: bool,
    /// Whether it's variadic (...$param)
    pub is_variadic: bool,
    /// Whether it's passed by reference
    pub is_reference: bool,
}

impl FunctionParameterInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_: None,
            is_optional: false,
            is_variadic: false,
            is_reference: false,
        }
    }

    pub fn with_type(mut self, ty: Type) -> Self {
        self.type_ = Some(ty);
        self
    }

    pub fn with_optional(mut self, is_optional: bool) -> Self {
        self.is_optional = is_optional;
        self
    }

    pub fn with_variadic(mut self, is_variadic: bool) -> Self {
        self.is_variadic = is_variadic;
        self
    }

    pub fn with_reference(mut self, is_reference: bool) -> Self {
        self.is_reference = is_reference;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_info_from_fqn() {
        let info = FunctionInfo::from_fqn("App\\Helpers\\format_date");
        assert_eq!(info.name, "format_date");
        assert_eq!(info.full_name, "App\\Helpers\\format_date");
        assert_eq!(info.namespace, Some("App\\Helpers".to_string()));
    }

    #[test]
    fn test_function_args() {
        let func = FunctionInfo::from_fqn("test")
            .with_parameter(FunctionParameterInfo::new("a"))
            .with_parameter(FunctionParameterInfo::new("b").with_optional(true))
            .with_parameter(FunctionParameterInfo::new("c").with_variadic(true));

        assert_eq!(func.required_args(), 1);
        assert_eq!(func.max_args(), None); // Variadic
        assert!(func.accepts_arg_count(1));
        assert!(func.accepts_arg_count(5));
        assert!(func.accepts_arg_count(100));
        assert!(!func.accepts_arg_count(0));
    }

    #[test]
    fn test_fixed_args() {
        let func = FunctionInfo::from_fqn("test")
            .with_parameter(FunctionParameterInfo::new("a"))
            .with_parameter(FunctionParameterInfo::new("b"));

        assert_eq!(func.required_args(), 2);
        assert_eq!(func.max_args(), Some(2));
        assert!(!func.accepts_arg_count(1));
        assert!(func.accepts_arg_count(2));
        assert!(!func.accepts_arg_count(3));
    }
}
