//! Class information for symbol table
//!
//! Stores metadata about classes for cross-file analysis.

use crate::types::Type;
use crate::types::php_type::Visibility;
use crate::types::phpdoc::TemplateParam;
use std::collections::HashMap;
use std::path::PathBuf;

/// Kind of class-like structure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    Class,
    Interface,
    Trait,
    Enum,
}

/// Information about a class stored in the symbol table
#[derive(Debug, Clone)]
pub struct ClassInfo {
    /// Short name (without namespace)
    pub name: String,
    /// Fully qualified name
    pub full_name: String,
    /// Namespace (if any)
    pub namespace: Option<String>,
    /// Kind of class-like structure
    pub kind: ClassKind,
    /// Parent class (if any)
    pub parent: Option<String>,
    /// Implemented interfaces
    pub interfaces: Vec<String>,
    /// Used traits
    pub traits: Vec<String>,
    /// Properties
    pub properties: HashMap<String, ClassPropertyInfo>,
    /// Methods
    pub methods: HashMap<String, ClassMethodInfo>,
    /// Constants
    pub constants: HashMap<String, ClassConstantInfo>,
    /// Whether it's abstract
    pub is_abstract: bool,
    /// Whether it's final
    pub is_final: bool,
    /// Whether it's readonly (PHP 8.2+)
    pub is_readonly: bool,
    /// File where this class is defined
    pub file: Option<PathBuf>,
    /// Line number where defined
    pub line: Option<usize>,
    /// Template parameters from @template annotations
    pub template_params: Vec<TemplateParam>,
}

impl ClassInfo {
    /// Create a new class info
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
            kind: ClassKind::Class,
            parent: None,
            interfaces: Vec::new(),
            traits: Vec::new(),
            properties: HashMap::new(),
            methods: HashMap::new(),
            constants: HashMap::new(),
            is_abstract: false,
            is_final: false,
            is_readonly: false,
            file: None,
            line: None,
            template_params: Vec::new(),
        }
    }

    /// Create from a fully qualified name
    pub fn from_fqn(fqn: impl Into<String>) -> Self {
        let fqn = fqn.into();
        let name = fqn.rsplit('\\').next().unwrap_or(&fqn).to_string();
        Self::new(name, fqn)
    }

    /// Check if the class has a method (case-insensitive)
    pub fn has_method(&self, name: &str) -> bool {
        self.methods.contains_key(&name.to_lowercase())
    }

    /// Get a method by name (case-insensitive)
    pub fn get_method(&self, name: &str) -> Option<&ClassMethodInfo> {
        self.methods.get(&name.to_lowercase())
    }

    /// Check if the class has a property
    pub fn has_property(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }

    /// Get a property by name
    pub fn get_property(&self, name: &str) -> Option<&ClassPropertyInfo> {
        self.properties.get(name)
    }

    /// Check if the class has a constant
    pub fn has_constant(&self, name: &str) -> bool {
        self.constants.contains_key(name)
    }

    /// Get a constant by name
    pub fn get_constant(&self, name: &str) -> Option<&ClassConstantInfo> {
        self.constants.get(name)
    }

    /// Add a method
    pub fn add_method(&mut self, method: ClassMethodInfo) {
        self.methods.insert(method.name.to_lowercase(), method);
    }

    /// Add a property
    pub fn add_property(&mut self, property: ClassPropertyInfo) {
        self.properties.insert(property.name.clone(), property);
    }

    /// Add a constant
    pub fn add_constant(&mut self, constant: ClassConstantInfo) {
        self.constants.insert(constant.name.clone(), constant);
    }

    /// Get the type that represents an instance of this class
    pub fn instance_type(&self) -> Type {
        Type::Object {
            class_name: Some(self.full_name.clone()),
        }
    }
}

/// Information about a class property
#[derive(Debug, Clone)]
pub struct ClassPropertyInfo {
    /// Property name (without $)
    pub name: String,
    /// Property type
    pub type_: Option<Type>,
    /// Visibility
    pub visibility: Visibility,
    /// Whether it's static
    pub is_static: bool,
    /// Whether it's readonly
    pub is_readonly: bool,
    /// Whether it has a default value
    pub has_default: bool,
}

impl ClassPropertyInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_: None,
            visibility: Visibility::Public,
            is_static: false,
            is_readonly: false,
            has_default: false,
        }
    }

    pub fn with_type(mut self, ty: Type) -> Self {
        self.type_ = Some(ty);
        self
    }

    pub fn with_visibility(mut self, vis: Visibility) -> Self {
        self.visibility = vis;
        self
    }

    pub fn with_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }
}

/// Information about a class method
#[derive(Debug, Clone)]
pub struct ClassMethodInfo {
    /// Method name
    pub name: String,
    /// Parameter info (name, type, is_optional, is_variadic)
    pub parameters: Vec<MethodParameterInfo>,
    /// Return type
    pub return_type: Option<Type>,
    /// Visibility
    pub visibility: Visibility,
    /// Whether it's static
    pub is_static: bool,
    /// Whether it's abstract
    pub is_abstract: bool,
    /// Whether it's final
    pub is_final: bool,
}

impl ClassMethodInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: Vec::new(),
            return_type: None,
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            is_final: false,
        }
    }

    pub fn with_return_type(mut self, ty: Type) -> Self {
        self.return_type = Some(ty);
        self
    }

    pub fn with_visibility(mut self, vis: Visibility) -> Self {
        self.visibility = vis;
        self
    }

    pub fn with_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }

    pub fn with_parameter(mut self, param: MethodParameterInfo) -> Self {
        self.parameters.push(param);
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
}

/// Information about a method parameter
#[derive(Debug, Clone)]
pub struct MethodParameterInfo {
    pub name: String,
    pub type_: Option<Type>,
    pub is_optional: bool,
    pub is_variadic: bool,
    pub is_reference: bool,
}

impl MethodParameterInfo {
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
}

/// Information about a class constant
#[derive(Debug, Clone)]
pub struct ClassConstantInfo {
    pub name: String,
    pub type_: Option<Type>,
    pub visibility: Visibility,
    pub is_final: bool,
}

impl ClassConstantInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_: None,
            visibility: Visibility::Public,
            is_final: false,
        }
    }

    pub fn with_type(mut self, ty: Type) -> Self {
        self.type_ = Some(ty);
        self
    }

    pub fn with_visibility(mut self, vis: Visibility) -> Self {
        self.visibility = vis;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_info_from_fqn() {
        let info = ClassInfo::from_fqn("App\\Models\\User");
        assert_eq!(info.name, "User");
        assert_eq!(info.full_name, "App\\Models\\User");
        assert_eq!(info.namespace, Some("App\\Models".to_string()));
    }

    #[test]
    fn test_method_lookup_case_insensitive() {
        let mut info = ClassInfo::from_fqn("Foo");
        info.add_method(ClassMethodInfo::new("getName"));

        assert!(info.has_method("getName"));
        assert!(info.has_method("getname"));
        assert!(info.has_method("GETNAME"));
    }

    #[test]
    fn test_method_args() {
        let method = ClassMethodInfo::new("test")
            .with_parameter(MethodParameterInfo::new("a"))
            .with_parameter(MethodParameterInfo::new("b").with_optional(true));

        assert_eq!(method.required_args(), 1);
        assert_eq!(method.max_args(), Some(2));
    }
}
