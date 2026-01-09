//! Scope tracking for variable types
//!
//! Manages variable type information at different points in the code.

use crate::types::Type;
use super::class_context::ClassContext;
use super::function_context::FunctionContext;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A scope in the analysis
///
/// Scopes track variable types and provide context for type-aware analysis.
/// Scopes can be nested (child scopes inherit from parent scopes).
#[derive(Debug, Clone)]
pub struct Scope {
    /// Variables defined in this scope: name -> type
    variables: HashMap<String, Type>,
    /// Class context (if inside a class)
    class_context: Option<Arc<ClassContext>>,
    /// Function context (if inside a function/method)
    function_context: Option<Arc<FunctionContext>>,
    /// Parent scope (for lookups)
    parent: Option<Arc<Scope>>,
    /// Whether this is a closure scope
    is_closure: bool,
    /// Variables inherited via closure `use`
    closure_bindings: HashSet<String>,
    /// Whether strict_types is enabled
    strict_types: bool,
    /// Current namespace
    namespace: Option<String>,
    /// Use imports: alias -> fully qualified name
    use_imports: HashMap<String, String>,
    /// Function use imports
    use_function_imports: HashMap<String, String>,
    /// Constant use imports
    use_const_imports: HashMap<String, String>,
}

impl Scope {
    /// Create a new root scope
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            class_context: None,
            function_context: None,
            parent: None,
            is_closure: false,
            closure_bindings: HashSet::new(),
            strict_types: false,
            namespace: None,
            use_imports: HashMap::new(),
            use_function_imports: HashMap::new(),
            use_const_imports: HashMap::new(),
        }
    }

    /// Create a child scope
    pub fn enter_scope(&self) -> Self {
        Self {
            variables: HashMap::new(),
            class_context: self.class_context.clone(),
            function_context: self.function_context.clone(),
            parent: Some(Arc::new(self.clone())),
            is_closure: false,
            closure_bindings: HashSet::new(),
            strict_types: self.strict_types,
            namespace: self.namespace.clone(),
            use_imports: self.use_imports.clone(),
            use_function_imports: self.use_function_imports.clone(),
            use_const_imports: self.use_const_imports.clone(),
        }
    }

    /// Create a closure scope
    pub fn enter_closure_scope(&self, bindings: HashSet<String>) -> Self {
        let mut scope = Self {
            variables: HashMap::new(),
            class_context: self.class_context.clone(),
            function_context: None, // New function context for closure
            parent: Some(Arc::new(self.clone())),
            is_closure: true,
            closure_bindings: bindings.clone(),
            strict_types: self.strict_types,
            namespace: self.namespace.clone(),
            use_imports: self.use_imports.clone(),
            use_function_imports: self.use_function_imports.clone(),
            use_const_imports: self.use_const_imports.clone(),
        };

        // Copy bound variables from parent
        for name in &bindings {
            if let Some(ty) = self.get_variable_type(name) {
                scope.variables.insert(name.clone(), ty);
            }
        }

        scope
    }

    /// Create a function scope
    pub fn enter_function_scope(&self, function_context: FunctionContext) -> Self {
        let mut scope = Self {
            variables: HashMap::new(),
            class_context: self.class_context.clone(),
            function_context: Some(Arc::new(function_context.clone())),
            parent: Some(Arc::new(self.clone())),
            is_closure: false,
            closure_bindings: HashSet::new(),
            strict_types: self.strict_types,
            namespace: self.namespace.clone(),
            use_imports: self.use_imports.clone(),
            use_function_imports: self.use_function_imports.clone(),
            use_const_imports: self.use_const_imports.clone(),
        };

        // Add parameters as variables
        for param in &function_context.parameters {
            scope.variables.insert(
                param.name.clone(),
                param.effective_type(),
            );
        }

        // Add $this for non-static methods
        if function_context.is_method && !function_context.is_static {
            if let Some(class_ctx) = &scope.class_context {
                scope.variables.insert("this".to_string(), class_ctx.this_type());
            }
        }

        scope
    }

    /// Create a class scope
    pub fn enter_class_scope(&self, class_context: ClassContext) -> Self {
        Self {
            variables: HashMap::new(),
            class_context: Some(Arc::new(class_context)),
            function_context: None,
            parent: Some(Arc::new(self.clone())),
            is_closure: false,
            closure_bindings: HashSet::new(),
            strict_types: self.strict_types,
            namespace: self.namespace.clone(),
            use_imports: self.use_imports.clone(),
            use_function_imports: self.use_function_imports.clone(),
            use_const_imports: self.use_const_imports.clone(),
        }
    }

    /// Get the type of a variable
    pub fn get_variable_type(&self, name: &str) -> Option<Type> {
        // Check local variables first
        if let Some(ty) = self.variables.get(name) {
            return Some(ty.clone());
        }

        // For closures, only check bound variables or parent if not a closure
        if self.is_closure {
            if self.closure_bindings.contains(name) {
                if let Some(parent) = &self.parent {
                    return parent.get_variable_type(name);
                }
            }
            return None;
        }

        // Check parent scope
        if let Some(parent) = &self.parent {
            return parent.get_variable_type(name);
        }

        None
    }

    /// Set a variable's type
    pub fn set_variable(&mut self, name: impl Into<String>, ty: Type) {
        self.variables.insert(name.into(), ty);
    }

    /// Check if a variable is defined
    pub fn has_variable(&self, name: &str) -> bool {
        self.get_variable_type(name).is_some()
    }

    /// Get all defined variable names
    pub fn defined_variables(&self) -> HashSet<String> {
        let mut vars = self.variables.keys().cloned().collect::<HashSet<_>>();
        if let Some(parent) = &self.parent {
            if !self.is_closure {
                vars.extend(parent.defined_variables());
            }
        }
        vars
    }

    /// Get the type of $this
    pub fn get_this_type(&self) -> Option<Type> {
        self.get_variable_type("this")
    }

    /// Check if we're inside a class
    pub fn is_in_class(&self) -> bool {
        self.class_context.is_some()
    }

    /// Check if we're inside a function
    pub fn is_in_function(&self) -> bool {
        self.function_context.is_some()
    }

    /// Check if we're inside a static method
    pub fn is_in_static_method(&self) -> bool {
        self.function_context
            .as_ref()
            .map_or(false, |f| f.is_static)
    }

    /// Get the current class context
    pub fn class_context(&self) -> Option<&ClassContext> {
        self.class_context.as_ref().map(|c| c.as_ref())
    }

    /// Get the current function context
    pub fn function_context(&self) -> Option<&FunctionContext> {
        self.function_context.as_ref().map(|f| f.as_ref())
    }

    /// Set strict_types mode
    pub fn set_strict_types(&mut self, strict: bool) {
        self.strict_types = strict;
    }

    /// Check if strict_types is enabled
    pub fn is_strict_types(&self) -> bool {
        self.strict_types
    }

    /// Set the namespace
    pub fn set_namespace(&mut self, ns: impl Into<String>) {
        self.namespace = Some(ns.into());
    }

    /// Get the current namespace
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Add a use import
    pub fn add_use_import(&mut self, alias: impl Into<String>, fqn: impl Into<String>) {
        self.use_imports.insert(alias.into(), fqn.into());
    }

    /// Resolve a class name using use imports
    pub fn resolve_class_name(&self, name: &str) -> String {
        // Already fully qualified
        if name.starts_with('\\') {
            return name[1..].to_string();
        }

        // Check use imports
        let first_part = name.split('\\').next().unwrap_or(name);
        if let Some(fqn) = self.use_imports.get(first_part) {
            if name.contains('\\') {
                // Partial match: use Foo\Bar; then Foo\Baz\Qux -> Bar\Baz\Qux
                let rest = &name[first_part.len()..];
                return format!("{}{}", fqn, rest);
            } else {
                return fqn.clone();
            }
        }

        // Prepend namespace
        if let Some(ns) = &self.namespace {
            format!("{}\\{}", ns, name)
        } else {
            name.to_string()
        }
    }

    /// Create a narrowed scope based on a condition being true
    ///
    /// This is used for type narrowing in conditionals.
    pub fn narrow_by_truthy(&self, var_name: &str, narrowed_type: Type) -> Self {
        let mut scope = self.clone();
        scope.variables.insert(var_name.to_string(), narrowed_type);
        scope
    }

    /// Remove a type from a variable (used for null checks)
    pub fn remove_type_from_variable(&self, var_name: &str, type_to_remove: &Type) -> Self {
        let mut scope = self.clone();
        if let Some(current_type) = self.get_variable_type(var_name) {
            let new_type = match (&current_type, type_to_remove) {
                (Type::Nullable(inner), Type::Null) => inner.as_ref().clone(),
                (Type::Union(types), Type::Null) => {
                    let filtered: Vec<_> = types
                        .iter()
                        .filter(|t| !matches!(t, Type::Null))
                        .cloned()
                        .collect();
                    if filtered.len() == 1 {
                        filtered.into_iter().next().unwrap()
                    } else if filtered.is_empty() {
                        Type::Never
                    } else {
                        Type::Union(filtered)
                    }
                }
                _ => current_type,
            };
            scope.variables.insert(var_name.to_string(), new_type);
        }
        scope
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_variable() {
        let mut scope = Scope::new();
        scope.set_variable("foo", Type::Int);
        assert_eq!(scope.get_variable_type("foo"), Some(Type::Int));
        assert!(scope.has_variable("foo"));
        assert!(!scope.has_variable("bar"));
    }

    #[test]
    fn test_scope_inheritance() {
        let mut parent = Scope::new();
        parent.set_variable("foo", Type::Int);

        let mut child = parent.enter_scope();
        child.set_variable("bar", Type::String);

        assert_eq!(child.get_variable_type("foo"), Some(Type::Int));
        assert_eq!(child.get_variable_type("bar"), Some(Type::String));
    }

    #[test]
    fn test_closure_scope_isolation() {
        let mut parent = Scope::new();
        parent.set_variable("foo", Type::Int);
        parent.set_variable("bar", Type::String);

        let bindings = vec!["foo".to_string()].into_iter().collect();
        let closure = parent.enter_closure_scope(bindings);

        // foo is bound, bar is not
        assert!(closure.has_variable("foo"));
        assert!(!closure.has_variable("bar"));
    }

    #[test]
    fn test_function_scope_params() {
        let scope = Scope::new();
        let func_ctx = FunctionContext::new("test")
            .with_parameter(
                super::super::function_context::ParameterInfo::new("a")
                    .with_type(Type::Int)
            );

        let func_scope = scope.enter_function_scope(func_ctx);
        assert_eq!(func_scope.get_variable_type("a"), Some(Type::Int));
    }

    #[test]
    fn test_resolve_class_name() {
        let mut scope = Scope::new();
        scope.set_namespace("App\\Models");
        scope.add_use_import("Collection", "Illuminate\\Support\\Collection");

        assert_eq!(scope.resolve_class_name("User"), "App\\Models\\User");
        assert_eq!(scope.resolve_class_name("Collection"), "Illuminate\\Support\\Collection");
        assert_eq!(scope.resolve_class_name("\\DateTime"), "DateTime");
    }

    #[test]
    fn test_type_narrowing() {
        let mut scope = Scope::new();
        scope.set_variable("x", Type::nullable(Type::String));

        let narrowed = scope.remove_type_from_variable("x", &Type::Null);
        assert_eq!(narrowed.get_variable_type("x"), Some(Type::String));
    }
}
