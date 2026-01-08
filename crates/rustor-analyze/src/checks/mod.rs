//! Static analysis checks organized by PHPStan level

pub mod level0;
pub mod level1;

use crate::config::PhpStanConfig;
use crate::issue::Issue;
use mago_syntax::ast::Program;
use std::path::Path;

/// Context provided to checks during analysis
pub struct CheckContext<'a> {
    /// The file being analyzed
    pub file_path: &'a Path,
    /// The source code
    pub source: &'a str,
    /// PHPStan configuration
    pub config: &'a PhpStanConfig,
    /// PHP built-in functions (for undefined function checks)
    pub builtin_functions: &'a [&'static str],
    /// PHP built-in classes
    pub builtin_classes: &'a [&'static str],
}

/// Trait for static analysis checks
pub trait Check: Send + Sync {
    /// Unique identifier for this check (e.g., "undefined.function")
    fn id(&self) -> &'static str;

    /// Human-readable description
    fn description(&self) -> &'static str;

    /// PHPStan level at which this check is enabled (0-9)
    fn level(&self) -> u8;

    /// Run the check and return any issues found
    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue>;
}

/// Registry of all available checks
#[derive(Default)]
pub struct CheckRegistry {
    checks: Vec<Box<dyn Check>>,
}

impl CheckRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with all built-in checks
    pub fn with_builtin_checks() -> Self {
        let mut registry = Self::new();

        // Level 0 checks
        registry.register(Box::new(level0::UndefinedFunctionCheck));
        registry.register(Box::new(level0::UndefinedClassCheck));

        // Level 1 checks
        registry.register(Box::new(level1::UndefinedVariableCheck));

        registry
    }

    /// Register a check
    pub fn register(&mut self, check: Box<dyn Check>) {
        self.checks.push(check);
    }

    /// Get all checks for a given level (includes all checks up to and including that level)
    pub fn checks_for_level(&self, level: u8) -> Vec<&dyn Check> {
        self.checks
            .iter()
            .filter(|c| c.level() <= level)
            .map(|c| c.as_ref())
            .collect()
    }

    /// Get all registered checks
    pub fn all_checks(&self) -> Vec<&dyn Check> {
        self.checks.iter().map(|c| c.as_ref()).collect()
    }
}

/// PHP built-in functions (partial list - most common ones)
pub const PHP_BUILTIN_FUNCTIONS: &[&str] = &[
    // String functions
    "strlen", "substr", "strpos", "stripos", "strrpos", "strripos", "str_replace", "str_ireplace",
    "strtolower", "strtoupper", "ucfirst", "lcfirst", "ucwords", "trim", "ltrim", "rtrim",
    "explode", "implode", "join", "sprintf", "printf", "sscanf", "vsprintf", "vprintf",
    "str_pad", "str_repeat", "str_split", "chunk_split", "wordwrap", "nl2br",
    "htmlspecialchars", "htmlentities", "strip_tags", "addslashes", "stripslashes",
    "quotemeta", "ord", "chr", "number_format", "money_format", "parse_str", "http_build_query",
    "preg_match", "preg_match_all", "preg_replace", "preg_split", "preg_grep",
    "str_contains", "str_starts_with", "str_ends_with",

    // Array functions
    "count", "sizeof", "array_push", "array_pop", "array_shift", "array_unshift",
    "array_merge", "array_merge_recursive", "array_combine", "array_chunk", "array_slice",
    "array_splice", "array_keys", "array_values", "array_flip", "array_reverse",
    "array_search", "array_key_exists", "in_array", "array_unique", "array_diff",
    "array_intersect", "array_map", "array_filter", "array_reduce", "array_walk",
    "array_column", "array_fill", "array_fill_keys", "array_pad", "range",
    "sort", "rsort", "asort", "arsort", "ksort", "krsort", "usort", "uasort", "uksort",
    "shuffle", "array_rand", "array_sum", "array_product", "array_count_values",
    "array_key_first", "array_key_last", "array_is_list",

    // File functions
    "file_get_contents", "file_put_contents", "file", "fopen", "fclose", "fread", "fwrite",
    "fgets", "fgetc", "feof", "fseek", "ftell", "rewind", "flock", "fflush",
    "file_exists", "is_file", "is_dir", "is_readable", "is_writable", "is_executable",
    "mkdir", "rmdir", "unlink", "rename", "copy", "move_uploaded_file",
    "glob", "scandir", "readdir", "opendir", "closedir", "realpath", "dirname", "basename",
    "pathinfo", "filesize", "filemtime", "fileatime", "filectime", "stat",

    // Type functions
    "gettype", "settype", "intval", "floatval", "strval", "boolval",
    "is_null", "is_bool", "is_int", "is_integer", "is_long", "is_float", "is_double",
    "is_string", "is_array", "is_object", "is_callable", "is_resource", "is_numeric",
    "is_scalar", "is_iterable", "is_countable", "isset", "unset", "empty",

    // Class/Object functions
    "class_exists", "interface_exists", "trait_exists", "method_exists", "property_exists",
    "get_class", "get_parent_class", "get_called_class", "get_class_methods", "get_class_vars",
    "get_object_vars", "is_a", "is_subclass_of", "instanceof",

    // Math functions
    "abs", "ceil", "floor", "round", "max", "min", "pow", "sqrt", "exp", "log", "log10",
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh",
    "deg2rad", "rad2deg", "pi", "fmod", "intdiv", "rand", "mt_rand", "random_int",

    // Date/Time functions
    "time", "mktime", "strtotime", "date", "gmdate", "strftime", "localtime", "getdate",
    "checkdate", "date_create", "date_format", "date_modify", "date_diff",

    // JSON functions
    "json_encode", "json_decode", "json_last_error", "json_last_error_msg",

    // Error handling
    "trigger_error", "user_error", "set_error_handler", "restore_error_handler",
    "set_exception_handler", "restore_exception_handler", "error_reporting",

    // Output
    "echo", "print", "print_r", "var_dump", "var_export", "debug_print_backtrace",
    "ob_start", "ob_end_clean", "ob_end_flush", "ob_get_contents", "ob_get_clean",

    // Variable functions
    "compact", "extract", "list", "define", "defined", "constant",

    // Misc
    "call_user_func", "call_user_func_array", "func_get_args", "func_get_arg", "func_num_args",
    "function_exists", "get_defined_functions", "create_function",
    "header", "headers_sent", "setcookie", "setrawcookie",
    "exit", "die", "sleep", "usleep", "flush",
    "phpinfo", "phpversion", "php_uname", "php_sapi_name",
    "serialize", "unserialize",
    "password_hash", "password_verify", "password_needs_rehash",
    "hash", "hash_hmac", "md5", "sha1", "crc32",
    "base64_encode", "base64_decode", "urlencode", "urldecode", "rawurlencode", "rawurldecode",
    "pack", "unpack",
    "assert",
    "debug_backtrace", "get_defined_vars",
];

/// PHP built-in classes (partial list)
pub const PHP_BUILTIN_CLASSES: &[&str] = &[
    // SPL
    "stdClass", "Exception", "Error", "TypeError", "ArgumentCountError", "ValueError",
    "RuntimeException", "LogicException", "InvalidArgumentException", "OutOfBoundsException",
    "OutOfRangeException", "OverflowException", "UnderflowException", "UnexpectedValueException",
    "DomainException", "RangeException", "LengthException", "BadMethodCallException",
    "BadFunctionCallException",

    // Iterators
    "Iterator", "IteratorAggregate", "ArrayIterator", "RecursiveIterator",
    "RecursiveArrayIterator", "DirectoryIterator", "RecursiveDirectoryIterator",
    "FilesystemIterator", "GlobIterator", "RegexIterator", "FilterIterator",
    "CallbackFilterIterator", "LimitIterator", "InfiniteIterator", "EmptyIterator",
    "AppendIterator", "MultipleIterator", "NoRewindIterator", "CachingIterator",

    // Data structures
    "ArrayObject", "SplFixedArray", "SplDoublyLinkedList", "SplStack", "SplQueue",
    "SplHeap", "SplMinHeap", "SplMaxHeap", "SplPriorityQueue", "SplObjectStorage",

    // File handling
    "SplFileInfo", "SplFileObject", "SplTempFileObject",

    // DateTime
    "DateTime", "DateTimeImmutable", "DateTimeZone", "DateInterval", "DatePeriod",
    "DateTimeInterface",

    // Reflection
    "ReflectionClass", "ReflectionMethod", "ReflectionProperty", "ReflectionFunction",
    "ReflectionParameter", "ReflectionType", "ReflectionNamedType", "ReflectionUnionType",
    "ReflectionAttribute", "ReflectionEnum", "ReflectionEnumUnitCase", "ReflectionEnumBackedCase",

    // Generators
    "Generator", "ClosedGeneratorException",

    // Closure
    "Closure",

    // Interfaces
    "Traversable", "Countable", "ArrayAccess", "Serializable", "JsonSerializable",
    "Stringable", "Throwable",

    // Attributes
    "Attribute", "ReturnTypeWillChange", "AllowDynamicProperties", "SensitiveParameter",

    // Weak references
    "WeakReference", "WeakMap",

    // Fibers (PHP 8.1+)
    "Fiber", "FiberError",

    // Enums (PHP 8.1+)
    "UnitEnum", "BackedEnum",

    // Random (PHP 8.2+)
    "Random\\Randomizer", "Random\\Engine", "Random\\Engine\\Mt19937",
    "Random\\Engine\\PcgOneseq128XslRr64", "Random\\Engine\\Xoshiro256StarStar",
    "Random\\Engine\\Secure",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_registry() {
        let registry = CheckRegistry::with_builtin_checks();
        let level0_checks = registry.checks_for_level(0);
        assert!(!level0_checks.is_empty());
    }
}
