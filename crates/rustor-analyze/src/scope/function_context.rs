//! Function context for scope tracking
//!
//! Represents the function/method context when analyzing code inside a function.

use crate::types::Type;

/// Information about a function parameter
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Parameter name (without $)
    pub name: String,
    /// Parameter type (if declared)
    pub type_: Option<Type>,
    /// Whether it has a default value (optional)
    pub is_optional: bool,
    /// Whether it's variadic (...$param)
    pub is_variadic: bool,
    /// Whether it's passed by reference (&$param)
    pub is_reference: bool,
    /// Whether it's promoted to a property (PHP 8.0+)
    pub is_promoted: bool,
    /// Default value as string (for constant default values)
    pub default_value: Option<String>,
}

impl ParameterInfo {
    /// Create a new parameter info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_: None,
            is_optional: false,
            is_variadic: false,
            is_reference: false,
            is_promoted: false,
            default_value: None,
        }
    }

    /// Set the type
    pub fn with_type(mut self, ty: Type) -> Self {
        self.type_ = Some(ty);
        self
    }

    /// Set as optional
    pub fn with_optional(mut self, is_optional: bool) -> Self {
        self.is_optional = is_optional;
        self
    }

    /// Set as variadic
    pub fn with_variadic(mut self, is_variadic: bool) -> Self {
        self.is_variadic = is_variadic;
        self
    }

    /// Set as reference
    pub fn with_reference(mut self, is_reference: bool) -> Self {
        self.is_reference = is_reference;
        self
    }

    /// Get the effective type for this parameter in the function body
    pub fn effective_type(&self) -> Type {
        if self.is_variadic {
            // Variadic parameters become arrays
            Type::Array {
                key: Box::new(Type::Int),
                value: Box::new(self.type_.clone().unwrap_or(Type::Mixed)),
            }
        } else if self.is_optional && self.type_.is_some() {
            // Optional parameters with type hints might be nullable
            // depending on the default value
            self.type_.clone().unwrap()
        } else {
            self.type_.clone().unwrap_or(Type::Mixed)
        }
    }
}

/// Context for code inside a function or method
#[derive(Debug, Clone)]
pub struct FunctionContext {
    /// Function name (empty for closures/arrow functions)
    pub name: String,
    /// Fully qualified name (with namespace/class)
    pub full_name: Option<String>,
    /// Function parameters
    pub parameters: Vec<ParameterInfo>,
    /// Declared return type
    pub return_type: Option<Type>,
    /// Whether it's a method (vs standalone function)
    pub is_method: bool,
    /// Whether it's a static method
    pub is_static: bool,
    /// Whether it's a closure
    pub is_closure: bool,
    /// Whether it's an arrow function
    pub is_arrow_function: bool,
    /// Whether it returns by reference
    pub returns_reference: bool,
    /// Variables captured by closure via `use`
    pub use_variables: Vec<(String, bool)>, // (name, is_reference)
}

impl FunctionContext {
    /// Create a new function context
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            full_name: None,
            parameters: Vec::new(),
            return_type: None,
            is_method: false,
            is_static: false,
            is_closure: false,
            is_arrow_function: false,
            returns_reference: false,
            use_variables: Vec::new(),
        }
    }

    /// Create a closure context
    pub fn new_closure() -> Self {
        let mut ctx = Self::new("");
        ctx.is_closure = true;
        ctx
    }

    /// Create an arrow function context
    pub fn new_arrow_function() -> Self {
        let mut ctx = Self::new("");
        ctx.is_arrow_function = true;
        ctx
    }

    /// Set the return type
    pub fn with_return_type(mut self, ty: Type) -> Self {
        self.return_type = Some(ty);
        self
    }

    /// Add a parameter
    pub fn with_parameter(mut self, param: ParameterInfo) -> Self {
        self.parameters.push(param);
        self
    }

    /// Set as method
    pub fn with_method(mut self, is_method: bool) -> Self {
        self.is_method = is_method;
        self
    }

    /// Set as static
    pub fn with_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }

    /// Add a use variable (for closures)
    pub fn with_use_variable(mut self, name: impl Into<String>, is_reference: bool) -> Self {
        self.use_variables.push((name.into(), is_reference));
        self
    }

    /// Get the minimum required parameter count
    pub fn required_param_count(&self) -> usize {
        self.parameters
            .iter()
            .take_while(|p| !p.is_optional && !p.is_variadic)
            .count()
    }

    /// Get the maximum parameter count (None if variadic)
    pub fn max_param_count(&self) -> Option<usize> {
        if self.parameters.iter().any(|p| p.is_variadic) {
            None
        } else {
            Some(self.parameters.len())
        }
    }

    /// Get parameter by name
    pub fn get_parameter(&self, name: &str) -> Option<&ParameterInfo> {
        self.parameters.iter().find(|p| p.name == name)
    }

    /// Get parameter by index
    pub fn get_parameter_by_index(&self, index: usize) -> Option<&ParameterInfo> {
        self.parameters.get(index)
    }

    /// Check if this function accepts N arguments
    pub fn accepts_arg_count(&self, count: usize) -> bool {
        let min = self.required_param_count();
        let max = self.max_param_count();

        count >= min && max.map_or(true, |m| count <= m)
    }
}

impl Default for FunctionContext {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_info() {
        let param = ParameterInfo::new("name")
            .with_type(Type::String)
            .with_optional(false);
        assert_eq!(param.name, "name");
        assert_eq!(param.type_, Some(Type::String));
        assert!(!param.is_optional);
    }

    #[test]
    fn test_variadic_effective_type() {
        let param = ParameterInfo::new("args")
            .with_type(Type::String)
            .with_variadic(true);
        let eff_type = param.effective_type();
        assert!(matches!(eff_type, Type::Array { .. }));
    }

    #[test]
    fn test_function_context() {
        let ctx = FunctionContext::new("doSomething")
            .with_parameter(ParameterInfo::new("a").with_type(Type::Int))
            .with_parameter(ParameterInfo::new("b").with_type(Type::String).with_optional(true))
            .with_return_type(Type::Bool);

        assert_eq!(ctx.required_param_count(), 1);
        assert_eq!(ctx.max_param_count(), Some(2));
        assert!(ctx.accepts_arg_count(1));
        assert!(ctx.accepts_arg_count(2));
        assert!(!ctx.accepts_arg_count(0));
        assert!(!ctx.accepts_arg_count(3));
    }

    #[test]
    fn test_variadic_function() {
        let ctx = FunctionContext::new("printf")
            .with_parameter(ParameterInfo::new("format").with_type(Type::String))
            .with_parameter(ParameterInfo::new("args").with_variadic(true));

        assert_eq!(ctx.required_param_count(), 1);
        assert_eq!(ctx.max_param_count(), None);
        assert!(ctx.accepts_arg_count(1));
        assert!(ctx.accepts_arg_count(10));
        assert!(ctx.accepts_arg_count(100));
    }
}
