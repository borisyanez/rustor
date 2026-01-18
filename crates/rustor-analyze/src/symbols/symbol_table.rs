//! Symbol table for cross-file analysis
//!
//! The symbol table collects information about classes, functions, and constants
//! from all analyzed files, enabling cross-file analysis.

use super::class_info::{ClassInfo, ClassKind};
use super::function_info::FunctionInfo;
use crate::types::Type;
use std::collections::HashMap;
use std::path::Path;

/// Symbol table containing all known symbols
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    /// Classes by fully qualified name (lowercase for case-insensitive lookup)
    classes: HashMap<String, ClassInfo>,
    /// Functions by fully qualified name (lowercase for case-insensitive lookup)
    functions: HashMap<String, FunctionInfo>,
    /// Constants by fully qualified name (case-sensitive)
    constants: HashMap<String, Type>,
    /// Namespace aliases: file path -> (alias -> fqn)
    namespace_aliases: HashMap<String, HashMap<String, String>>,
}

impl SymbolTable {
    /// Create a new empty symbol table
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a symbol table with PHP built-in symbols
    pub fn with_builtins() -> Self {
        let mut table = Self::new();
        table.register_builtins();
        table
    }

    /// Register a class
    pub fn register_class(&mut self, info: ClassInfo) {
        let key = info.full_name.to_lowercase();
        self.classes.insert(key, info);
    }

    /// Register a function
    pub fn register_function(&mut self, info: FunctionInfo) {
        let key = info.full_name.to_lowercase();
        self.functions.insert(key, info);
    }

    /// Register a constant
    pub fn register_constant(&mut self, name: impl Into<String>, type_: Type) {
        self.constants.insert(name.into(), type_);
    }

    /// Get a class by fully qualified name (case-insensitive)
    pub fn get_class(&self, name: &str) -> Option<&ClassInfo> {
        self.classes.get(&name.to_lowercase())
    }

    /// Get a function by fully qualified name (case-insensitive)
    pub fn get_function(&self, name: &str) -> Option<&FunctionInfo> {
        self.functions.get(&name.to_lowercase())
    }

    /// Get a constant by name (case-sensitive)
    pub fn get_constant(&self, name: &str) -> Option<&Type> {
        self.constants.get(name)
    }

    /// Check if a class exists
    pub fn class_exists(&self, name: &str) -> bool {
        self.classes.contains_key(&name.to_lowercase())
    }

    /// Check if a function exists
    pub fn function_exists(&self, name: &str) -> bool {
        self.functions.contains_key(&name.to_lowercase())
    }

    /// Check if a constant exists
    pub fn constant_exists(&self, name: &str) -> bool {
        self.constants.contains_key(name)
    }

    /// Check if a class has a method
    pub fn class_has_method(&self, class: &str, method: &str) -> bool {
        self.get_class(class)
            .map_or(false, |c| c.has_method(method))
    }

    /// Check if a class has a property
    pub fn class_has_property(&self, class: &str, property: &str) -> bool {
        self.get_class(class)
            .map_or(false, |c| c.has_property(property))
    }

    /// Check if a class has a constant
    pub fn class_has_constant(&self, class: &str, constant: &str) -> bool {
        self.get_class(class)
            .map_or(false, |c| c.has_constant(constant))
    }

    /// Get all class names
    pub fn all_classes(&self) -> impl Iterator<Item = &str> {
        self.classes.values().map(|c| c.full_name.as_str())
    }

    /// Get all function names
    pub fn all_functions(&self) -> impl Iterator<Item = &str> {
        self.functions.values().map(|f| f.full_name.as_str())
    }

    /// Store namespace aliases for a file
    pub fn set_aliases(&mut self, file: &Path, aliases: HashMap<String, String>) {
        self.namespace_aliases
            .insert(file.to_string_lossy().to_string(), aliases);
    }

    /// Get namespace aliases for a file
    pub fn get_aliases(&self, file: &Path) -> Option<&HashMap<String, String>> {
        self.namespace_aliases.get(&file.to_string_lossy().to_string())
    }

    /// Resolve a class name in a given file context
    pub fn resolve_class_name(&self, name: &str, file: &Path, namespace: Option<&str>) -> String {
        // Already fully qualified
        if name.starts_with('\\') {
            return name[1..].to_string();
        }

        // Check file aliases (case-insensitive lookup since PHP class names are case-insensitive)
        if let Some(aliases) = self.get_aliases(file) {
            let first_part = name.split('\\').next().unwrap_or(name);
            let first_part_lower = first_part.to_lowercase();

            // Case-insensitive alias lookup
            for (alias_key, fqn) in aliases {
                if alias_key.to_lowercase() == first_part_lower {
                    if name.contains('\\') {
                        let rest = &name[first_part.len()..];
                        return format!("{}{}", fqn, rest);
                    } else {
                        return fqn.clone();
                    }
                }
            }
        }

        // Prepend namespace
        if let Some(ns) = namespace {
            format!("{}\\{}", ns, name)
        } else {
            name.to_string()
        }
    }

    /// Register PHP built-in classes and functions
    fn register_builtins(&mut self) {
        // Register common built-in classes with inheritance
        // Format: (name, kind, parent, interfaces)
        let builtin_classes_with_hierarchy: &[(&str, ClassKind, Option<&str>, &[&str])] = &[
            ("stdClass", ClassKind::Class, None, &[]),
            ("Exception", ClassKind::Class, None, &["Throwable"]),
            ("Error", ClassKind::Class, None, &["Throwable"]),
            ("TypeError", ClassKind::Class, Some("Error"), &[]),
            ("ArgumentCountError", ClassKind::Class, Some("TypeError"), &[]),
            ("ValueError", ClassKind::Class, Some("Error"), &[]),
            ("RuntimeException", ClassKind::Class, Some("Exception"), &[]),
            ("LogicException", ClassKind::Class, Some("Exception"), &[]),
            ("InvalidArgumentException", ClassKind::Class, Some("LogicException"), &[]),
            ("OutOfBoundsException", ClassKind::Class, Some("RuntimeException"), &[]),
            ("OutOfRangeException", ClassKind::Class, Some("RuntimeException"), &[]),
            ("UnexpectedValueException", ClassKind::Class, Some("RuntimeException"), &[]),
            ("DomainException", ClassKind::Class, Some("LogicException"), &[]),
            ("LengthException", ClassKind::Class, Some("LogicException"), &[]),
            ("RangeException", ClassKind::Class, Some("RuntimeException"), &[]),
            ("OverflowException", ClassKind::Class, Some("RuntimeException"), &[]),
            ("UnderflowException", ClassKind::Class, Some("RuntimeException"), &[]),
            ("BadMethodCallException", ClassKind::Class, Some("BadFunctionCallException"), &[]),
            ("BadFunctionCallException", ClassKind::Class, Some("LogicException"), &[]),
        ];

        for (name, kind, parent, interfaces) in builtin_classes_with_hierarchy {
            let mut info = ClassInfo::from_fqn(*name);
            info.kind = *kind;
            if let Some(p) = parent {
                info.parent = Some(p.to_string());
            }
            for iface in *interfaces {
                info.interfaces.push(iface.to_string());
            }
            self.register_class(info);
        }

        // Register other built-in classes without specific hierarchy
        let builtin_classes = [
            ("DateTime", ClassKind::Class),
            ("DateTimeImmutable", ClassKind::Class),
            ("DateTimeZone", ClassKind::Class),
            ("DateInterval", ClassKind::Class),
            ("ArrayObject", ClassKind::Class),
            ("ArrayIterator", ClassKind::Class),
            ("Iterator", ClassKind::Interface),
            ("IteratorAggregate", ClassKind::Interface),
            ("Traversable", ClassKind::Interface),
            ("Countable", ClassKind::Interface),
            ("ArrayAccess", ClassKind::Interface),
            ("Serializable", ClassKind::Interface),
            ("JsonSerializable", ClassKind::Interface),
            ("Stringable", ClassKind::Interface),
            ("Throwable", ClassKind::Interface),
            ("Closure", ClassKind::Class),
            ("Generator", ClassKind::Class),
            ("ReflectionClass", ClassKind::Class),
            ("ReflectionMethod", ClassKind::Class),
            ("ReflectionProperty", ClassKind::Class),
            ("ReflectionFunction", ClassKind::Class),
            ("PDO", ClassKind::Class),
            ("PDOStatement", ClassKind::Class),
            ("PDOException", ClassKind::Class),
            ("SplFileInfo", ClassKind::Class),
            ("SplFileObject", ClassKind::Class),
            ("SplObjectStorage", ClassKind::Class),
            ("WeakReference", ClassKind::Class),
            ("WeakMap", ClassKind::Class),
            ("Fiber", ClassKind::Class),
            ("UnitEnum", ClassKind::Interface),
            ("BackedEnum", ClassKind::Interface),
            ("DOMDocument", ClassKind::Class),
            ("DOMElement", ClassKind::Class),
            ("DOMNode", ClassKind::Class),
            ("SimpleXMLElement", ClassKind::Class),
        ];

        for (name, kind) in builtin_classes {
            let mut info = ClassInfo::from_fqn(name);
            info.kind = kind;
            self.register_class(info);
        }

        // Register common built-in functions with signatures
        // This is a simplified version - in production you'd want full signatures
        let builtin_functions = [
            "strlen", "substr", "strpos", "str_replace", "explode", "implode",
            "array_map", "array_filter", "array_reduce", "array_merge", "array_keys", "array_values",
            "count", "sizeof", "in_array", "array_search", "array_key_exists",
            "is_null", "is_array", "is_string", "is_int", "is_float", "is_bool", "is_object",
            "isset", "empty", "unset",
            "print_r", "var_dump", "var_export",
            "json_encode", "json_decode",
            "file_get_contents", "file_put_contents", "file_exists", "is_file", "is_dir",
            "preg_match", "preg_match_all", "preg_replace",
            "sprintf", "printf", "sscanf",
            "trim", "ltrim", "rtrim", "strtolower", "strtoupper",
            "abs", "ceil", "floor", "round", "max", "min",
            "date", "time", "strtotime", "mktime",
            "class_exists", "method_exists", "property_exists", "function_exists",
            "get_class", "get_parent_class", "is_a", "is_subclass_of",
            "call_user_func", "call_user_func_array",
        ];

        for name in builtin_functions {
            let info = FunctionInfo::from_fqn(name);
            self.register_function(info);
        }

        // Register built-in constants
        self.register_constant("PHP_VERSION", Type::String);
        self.register_constant("PHP_INT_MAX", Type::Int);
        self.register_constant("PHP_INT_MIN", Type::Int);
        self.register_constant("PHP_EOL", Type::String);
        self.register_constant("DIRECTORY_SEPARATOR", Type::String);
        self.register_constant("PATH_SEPARATOR", Type::String);
        self.register_constant("NULL", Type::Null);
        self.register_constant("TRUE", Type::ConstantBool(true));
        self.register_constant("FALSE", Type::ConstantBool(false));
    }

    /// Merge another symbol table into this one
    pub fn merge(&mut self, other: SymbolTable) {
        self.classes.extend(other.classes);
        self.functions.extend(other.functions);
        self.constants.extend(other.constants);
        self.namespace_aliases.extend(other.namespace_aliases);
    }

    /// Get statistics about the symbol table
    pub fn stats(&self) -> SymbolTableStats {
        SymbolTableStats {
            class_count: self.classes.len(),
            function_count: self.functions.len(),
            constant_count: self.constants.len(),
        }
    }
}

/// Statistics about the symbol table
#[derive(Debug, Clone)]
pub struct SymbolTableStats {
    pub class_count: usize,
    pub function_count: usize,
    pub constant_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_lookup_class() {
        let mut table = SymbolTable::new();
        let class = ClassInfo::from_fqn("App\\Models\\User");
        table.register_class(class);

        assert!(table.class_exists("App\\Models\\User"));
        assert!(table.class_exists("app\\models\\user")); // Case insensitive
        assert!(!table.class_exists("App\\Models\\Post"));
    }

    #[test]
    fn test_register_and_lookup_function() {
        let mut table = SymbolTable::new();
        let func = FunctionInfo::from_fqn("App\\Helpers\\format_date");
        table.register_function(func);

        assert!(table.function_exists("App\\Helpers\\format_date"));
        assert!(table.function_exists("app\\helpers\\format_date")); // Case insensitive
    }

    #[test]
    fn test_builtins() {
        let table = SymbolTable::with_builtins();

        assert!(table.class_exists("DateTime"));
        assert!(table.class_exists("Exception"));
        assert!(table.function_exists("strlen"));
        assert!(table.function_exists("array_map"));
        assert!(table.constant_exists("PHP_VERSION"));
    }

    #[test]
    fn test_class_method_property() {
        use crate::symbols::class_info::{ClassMethodInfo, ClassPropertyInfo};

        let mut table = SymbolTable::new();
        let mut class = ClassInfo::from_fqn("Foo");
        class.add_method(ClassMethodInfo::new("bar"));
        class.add_property(ClassPropertyInfo::new("baz"));
        table.register_class(class);

        assert!(table.class_has_method("Foo", "bar"));
        assert!(table.class_has_method("Foo", "BAR")); // Case insensitive
        assert!(table.class_has_property("Foo", "baz"));
        assert!(!table.class_has_property("Foo", "BAZ")); // Properties are case sensitive
    }
}
