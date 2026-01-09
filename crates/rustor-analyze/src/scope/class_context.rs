//! Class context for scope tracking
//!
//! Represents the class context when analyzing code inside a class definition.

use crate::types::Type;
use crate::types::php_type::Visibility;
use std::collections::HashMap;

/// Information about a class property
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    /// Property name (without $)
    pub name: String,
    /// Property type (if declared or inferred)
    pub type_: Option<Type>,
    /// Visibility modifier
    pub visibility: Visibility,
    /// Whether it's a static property
    pub is_static: bool,
    /// Whether it's readonly (PHP 8.1+)
    pub is_readonly: bool,
    /// Whether it has a default value
    pub has_default: bool,
    /// Whether it's promoted from constructor
    pub is_promoted: bool,
}

impl PropertyInfo {
    /// Create a new property info with minimal information
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_: None,
            visibility: Visibility::Public,
            is_static: false,
            is_readonly: false,
            has_default: false,
            is_promoted: false,
        }
    }

    /// Set the type
    pub fn with_type(mut self, ty: Type) -> Self {
        self.type_ = Some(ty);
        self
    }

    /// Set visibility
    pub fn with_visibility(mut self, vis: Visibility) -> Self {
        self.visibility = vis;
        self
    }

    /// Set as static
    pub fn with_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }

    /// Set as readonly
    pub fn with_readonly(mut self, is_readonly: bool) -> Self {
        self.is_readonly = is_readonly;
        self
    }
}

/// Information about a class method
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Method name
    pub name: String,
    /// Method parameters
    pub parameters: Vec<crate::scope::function_context::ParameterInfo>,
    /// Return type (if declared or inferred)
    pub return_type: Option<Type>,
    /// Visibility modifier
    pub visibility: Visibility,
    /// Whether it's a static method
    pub is_static: bool,
    /// Whether it's abstract
    pub is_abstract: bool,
    /// Whether it's final
    pub is_final: bool,
    /// Whether it returns by reference
    pub returns_reference: bool,
}

impl MethodInfo {
    /// Create a new method info with minimal information
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: Vec::new(),
            return_type: None,
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            is_final: false,
            returns_reference: false,
        }
    }

    /// Set the return type
    pub fn with_return_type(mut self, ty: Type) -> Self {
        self.return_type = Some(ty);
        self
    }

    /// Set visibility
    pub fn with_visibility(mut self, vis: Visibility) -> Self {
        self.visibility = vis;
        self
    }

    /// Set as static
    pub fn with_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }

    /// Add a parameter
    pub fn with_parameter(mut self, param: crate::scope::function_context::ParameterInfo) -> Self {
        self.parameters.push(param);
        self
    }

    /// Get the minimum required parameter count
    pub fn required_param_count(&self) -> usize {
        self.parameters.iter().take_while(|p| !p.is_optional && !p.is_variadic).count()
    }

    /// Get the maximum parameter count (None if variadic)
    pub fn max_param_count(&self) -> Option<usize> {
        if self.parameters.iter().any(|p| p.is_variadic) {
            None
        } else {
            Some(self.parameters.len())
        }
    }
}

/// Context for code inside a class
#[derive(Debug, Clone)]
pub struct ClassContext {
    /// Fully qualified class name
    pub name: String,
    /// Short name (without namespace)
    pub short_name: String,
    /// Parent class (if extends)
    pub parent: Option<String>,
    /// Implemented interfaces
    pub interfaces: Vec<String>,
    /// Used traits
    pub traits: Vec<String>,
    /// Class properties
    pub properties: HashMap<String, PropertyInfo>,
    /// Class methods
    pub methods: HashMap<String, MethodInfo>,
    /// Class constants
    pub constants: HashMap<String, Type>,
    /// Whether it's an abstract class
    pub is_abstract: bool,
    /// Whether it's a final class
    pub is_final: bool,
    /// Whether it's an interface
    pub is_interface: bool,
    /// Whether it's a trait
    pub is_trait: bool,
    /// Whether it's an enum
    pub is_enum: bool,
    /// Whether it's readonly (PHP 8.2+)
    pub is_readonly: bool,
}

impl ClassContext {
    /// Create a new class context
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let short_name = name.rsplit('\\').next().unwrap_or(&name).to_string();
        Self {
            name,
            short_name,
            parent: None,
            interfaces: Vec::new(),
            traits: Vec::new(),
            properties: HashMap::new(),
            methods: HashMap::new(),
            constants: HashMap::new(),
            is_abstract: false,
            is_final: false,
            is_interface: false,
            is_trait: false,
            is_enum: false,
            is_readonly: false,
        }
    }

    /// Create an interface context
    pub fn new_interface(name: impl Into<String>) -> Self {
        let mut ctx = Self::new(name);
        ctx.is_interface = true;
        ctx
    }

    /// Create a trait context
    pub fn new_trait(name: impl Into<String>) -> Self {
        let mut ctx = Self::new(name);
        ctx.is_trait = true;
        ctx
    }

    /// Create an enum context
    pub fn new_enum(name: impl Into<String>) -> Self {
        let mut ctx = Self::new(name);
        ctx.is_enum = true;
        ctx
    }

    /// Set the parent class
    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    /// Add an interface
    pub fn with_interface(mut self, interface: impl Into<String>) -> Self {
        self.interfaces.push(interface.into());
        self
    }

    /// Add a property
    pub fn with_property(mut self, prop: PropertyInfo) -> Self {
        self.properties.insert(prop.name.clone(), prop);
        self
    }

    /// Add a method
    pub fn with_method(mut self, method: MethodInfo) -> Self {
        self.methods.insert(method.name.to_lowercase(), method);
        self
    }

    /// Check if the class has a property (case-sensitive)
    pub fn has_property(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }

    /// Check if the class has a method (case-insensitive)
    pub fn has_method(&self, name: &str) -> bool {
        self.methods.contains_key(&name.to_lowercase())
    }

    /// Get a property by name
    pub fn get_property(&self, name: &str) -> Option<&PropertyInfo> {
        self.properties.get(name)
    }

    /// Get a method by name (case-insensitive)
    pub fn get_method(&self, name: &str) -> Option<&MethodInfo> {
        self.methods.get(&name.to_lowercase())
    }

    /// Check if a property is accessible from the given context
    pub fn can_access_property(&self, prop: &PropertyInfo, from_class: Option<&str>) -> bool {
        match prop.visibility {
            Visibility::Public => true,
            Visibility::Protected => {
                from_class.map_or(false, |c| {
                    c.eq_ignore_ascii_case(&self.name)
                        || self.parent.as_ref().map_or(false, |p| p.eq_ignore_ascii_case(c))
                })
            }
            Visibility::Private => {
                from_class.map_or(false, |c| c.eq_ignore_ascii_case(&self.name))
            }
        }
    }

    /// Check if a method is accessible from the given context
    pub fn can_access_method(&self, method: &MethodInfo, from_class: Option<&str>) -> bool {
        match method.visibility {
            Visibility::Public => true,
            Visibility::Protected => {
                from_class.map_or(false, |c| {
                    c.eq_ignore_ascii_case(&self.name)
                        || self.parent.as_ref().map_or(false, |p| p.eq_ignore_ascii_case(c))
                })
            }
            Visibility::Private => {
                from_class.map_or(false, |c| c.eq_ignore_ascii_case(&self.name))
            }
        }
    }

    /// Get the type of $this in this class context
    pub fn this_type(&self) -> Type {
        Type::Object {
            class_name: Some(self.name.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_context_creation() {
        let ctx = ClassContext::new("App\\Models\\User");
        assert_eq!(ctx.name, "App\\Models\\User");
        assert_eq!(ctx.short_name, "User");
    }

    #[test]
    fn test_property_info() {
        let prop = PropertyInfo::new("name")
            .with_type(Type::String)
            .with_visibility(Visibility::Private);
        assert_eq!(prop.name, "name");
        assert_eq!(prop.type_, Some(Type::String));
        assert_eq!(prop.visibility, Visibility::Private);
    }

    #[test]
    fn test_method_info() {
        let method = MethodInfo::new("getName")
            .with_return_type(Type::String)
            .with_visibility(Visibility::Public);
        assert_eq!(method.name, "getName");
        assert_eq!(method.return_type, Some(Type::String));
    }

    #[test]
    fn test_has_method_case_insensitive() {
        let ctx = ClassContext::new("Foo")
            .with_method(MethodInfo::new("getName"));
        assert!(ctx.has_method("getName"));
        assert!(ctx.has_method("getname"));
        assert!(ctx.has_method("GETNAME"));
    }
}
