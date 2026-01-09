//! Static analysis checks organized by PHPStan level

pub mod level0;
pub mod level1;
pub mod level2;

use crate::config::PhpStanConfig;
use crate::issue::Issue;
use crate::scope::Scope;
use crate::symbols::SymbolTable;
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
    /// Symbol table for cross-file analysis (optional)
    pub symbol_table: Option<&'a SymbolTable>,
    /// Current scope for variable tracking (optional)
    pub scope: Option<&'a Scope>,
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
        registry.register(Box::new(level0::CallStaticMethodsCheck));
        registry.register(Box::new(level0::ClassConstantCheck));
        registry.register(Box::new(level0::ArgumentCountCheck));

        // Level 1 checks
        registry.register(Box::new(level1::UndefinedVariableCheck));

        // Level 2 checks
        registry.register(Box::new(level2::CallMethodsCheck));
        registry.register(Box::new(level2::PropertyAccessCheck));

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

/// PHP built-in functions (comprehensive list)
pub const PHP_BUILTIN_FUNCTIONS: &[&str] = &[
    // String functions
    "strlen", "substr", "strpos", "stripos", "strrpos", "strripos", "str_replace", "str_ireplace",
    "strtolower", "strtoupper", "ucfirst", "lcfirst", "ucwords", "trim", "ltrim", "rtrim",
    "explode", "implode", "join", "sprintf", "printf", "sscanf", "vsprintf", "vprintf",
    "str_pad", "str_repeat", "str_split", "chunk_split", "wordwrap", "nl2br",
    "htmlspecialchars", "htmlentities", "html_entity_decode", "htmlspecialchars_decode",
    "strip_tags", "addslashes", "stripslashes", "addcslashes", "stripcslashes",
    "quotemeta", "ord", "chr", "number_format", "money_format", "parse_str", "http_build_query",
    "preg_match", "preg_match_all", "preg_replace", "preg_replace_callback", "preg_replace_callback_array",
    "preg_split", "preg_grep", "preg_quote", "preg_last_error", "preg_last_error_msg",
    "str_contains", "str_starts_with", "str_ends_with", "str_getcsv", "str_rot13", "str_shuffle",
    "str_word_count", "strcasecmp", "strcmp", "strcoll", "strcspn", "strspn",
    "stristr", "strstr", "strrchr", "strrev", "strtok", "strtr", "substr_compare",
    "substr_count", "substr_replace", "levenshtein", "similar_text", "soundex", "metaphone",
    "quoted_printable_encode", "quoted_printable_decode", "convert_uuencode", "convert_uudecode",
    "wordwrap", "fprintf", "vfprintf", "setlocale", "localeconv",

    // Multibyte string functions
    "mb_strlen", "mb_substr", "mb_strpos", "mb_stripos", "mb_strrpos", "mb_strripos",
    "mb_strtolower", "mb_strtoupper", "mb_convert_case", "mb_convert_encoding",
    "mb_detect_encoding", "mb_check_encoding", "mb_encode_mimeheader", "mb_decode_mimeheader",
    "mb_internal_encoding", "mb_language", "mb_send_mail", "mb_get_info",
    "mb_http_input", "mb_http_output", "mb_output_handler", "mb_preferred_mime_name",
    "mb_regex_encoding", "mb_regex_set_options", "mb_ereg", "mb_eregi", "mb_ereg_replace",
    "mb_eregi_replace", "mb_split", "mb_ereg_match", "mb_ereg_search", "mb_ereg_search_pos",
    "mb_ereg_search_regs", "mb_ereg_search_init", "mb_ereg_search_getregs", "mb_ereg_search_getpos",
    "mb_ereg_search_setpos", "mb_substitute_character", "mb_parse_str", "mb_ord", "mb_chr",
    "mb_scrub", "mb_str_split", "mb_trim", "mb_ltrim", "mb_rtrim",

    // Array functions
    "count", "sizeof", "array_push", "array_pop", "array_shift", "array_unshift",
    "array_merge", "array_merge_recursive", "array_replace", "array_replace_recursive",
    "array_combine", "array_chunk", "array_slice", "array_splice",
    "array_keys", "array_values", "array_flip", "array_reverse",
    "array_search", "array_key_exists", "key_exists", "in_array", "array_unique", "array_diff",
    "array_diff_key", "array_diff_ukey", "array_diff_assoc", "array_diff_uassoc", "array_udiff",
    "array_udiff_assoc", "array_udiff_uassoc", "array_intersect", "array_intersect_key",
    "array_intersect_ukey", "array_intersect_assoc", "array_intersect_uassoc", "array_uintersect",
    "array_uintersect_assoc", "array_uintersect_uassoc",
    "array_map", "array_filter", "array_reduce", "array_walk", "array_walk_recursive",
    "array_column", "array_fill", "array_fill_keys", "array_pad", "range",
    "sort", "rsort", "asort", "arsort", "ksort", "krsort", "usort", "uasort", "uksort",
    "natsort", "natcasesort", "shuffle", "array_rand", "array_sum", "array_product",
    "array_count_values", "array_key_first", "array_key_last", "array_is_list",
    "array_multisort", "array_change_key_case", "current", "key", "next", "prev", "reset", "end",
    "each", "list", "array_all", "array_any", "array_find", "array_find_key",

    // File functions
    "file_get_contents", "file_put_contents", "file", "fopen", "fclose", "fread", "fwrite",
    "fgets", "fgetc", "fgetss", "fgetcsv", "fputcsv", "fpassthru", "ftruncate",
    "feof", "fseek", "ftell", "rewind", "flock", "fflush", "fstat",
    "file_exists", "is_file", "is_dir", "is_link", "is_readable", "is_writable", "is_writeable",
    "is_executable", "is_uploaded_file", "mkdir", "rmdir", "unlink", "rename", "copy",
    "move_uploaded_file", "tempnam", "tmpfile", "touch", "chmod", "chown", "chgrp",
    "glob", "scandir", "readdir", "opendir", "closedir", "rewinddir", "dir",
    "realpath", "dirname", "basename", "pathinfo", "parse_url",
    "filesize", "filemtime", "fileatime", "filectime", "fileinode", "fileowner", "filegroup",
    "fileperms", "filetype", "stat", "lstat", "clearstatcache", "disk_free_space", "disk_total_space",
    "readfile", "readlink", "symlink", "link", "linkinfo",
    "popen", "pclose", "proc_open", "proc_close", "proc_get_status", "proc_terminate",
    "stream_context_create", "stream_context_set_option", "stream_context_get_options",
    "stream_get_contents", "stream_get_meta_data", "stream_set_blocking", "stream_set_timeout",
    "stream_socket_client", "stream_socket_server", "stream_socket_accept",
    "stream_copy_to_stream", "stream_filter_append", "stream_filter_prepend", "stream_filter_remove",
    "stream_select", "stream_set_chunk_size", "stream_set_read_buffer", "stream_set_write_buffer",
    "stream_supports_lock", "stream_is_local", "stream_isatty", "stream_resolve_include_path",
    "finfo_open", "finfo_close", "finfo_file", "finfo_buffer", "finfo_set_flags",
    "mime_content_type",

    // Type functions
    "gettype", "settype", "intval", "floatval", "strval", "boolval",
    "is_null", "is_bool", "is_int", "is_integer", "is_long", "is_float", "is_double", "is_real",
    "is_string", "is_array", "is_object", "is_callable", "is_resource", "is_numeric",
    "is_scalar", "is_iterable", "is_countable", "isset", "unset", "empty",
    "get_debug_type", "get_resource_type", "get_resource_id",

    // Class/Object functions
    "class_exists", "interface_exists", "trait_exists", "enum_exists", "method_exists", "property_exists",
    "get_class", "get_parent_class", "get_called_class", "get_class_methods", "get_class_vars",
    "get_object_vars", "get_mangled_object_vars", "is_a", "is_subclass_of",
    "class_alias", "class_implements", "class_parents", "class_uses",
    "get_declared_classes", "get_declared_interfaces", "get_declared_traits",
    "spl_autoload", "spl_autoload_register", "spl_autoload_unregister", "spl_autoload_functions",
    "spl_autoload_extensions", "spl_autoload_call", "spl_classes", "spl_object_hash", "spl_object_id",

    // Math functions
    "abs", "ceil", "floor", "round", "max", "min", "pow", "sqrt", "exp", "expm1", "log", "log10",
    "log1p", "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh",
    "asinh", "acosh", "atanh", "deg2rad", "rad2deg", "pi", "fmod", "intdiv", "fdiv",
    "rand", "srand", "getrandmax", "mt_rand", "mt_srand", "mt_getrandmax", "random_int", "random_bytes",
    "base_convert", "bindec", "octdec", "hexdec", "decbin", "decoct", "dechex",
    "is_nan", "is_infinite", "is_finite", "hypot", "fma",

    // Date/Time functions
    "time", "mktime", "gmmktime", "strtotime", "date", "gmdate", "idate", "getdate", "localtime",
    "strftime", "gmstrftime", "checkdate", "microtime", "gettimeofday", "hrtime",
    "date_create", "date_create_immutable", "date_create_from_format", "date_create_immutable_from_format",
    "date_parse", "date_parse_from_format", "date_format", "date_modify", "date_add", "date_sub",
    "date_diff", "date_timestamp_get", "date_timestamp_set", "date_timezone_get", "date_timezone_set",
    "date_offset_get", "date_time_set", "date_date_set", "date_isodate_set",
    "date_default_timezone_get", "date_default_timezone_set", "timezone_open", "timezone_name_get",
    "timezone_name_from_abbr", "timezone_offset_get", "timezone_transitions_get", "timezone_location_get",
    "timezone_identifiers_list", "timezone_abbreviations_list", "timezone_version_get",
    "date_interval_create_from_date_string", "date_interval_format",
    "date_sunrise", "date_sunset", "date_sun_info",

    // JSON functions
    "json_encode", "json_decode", "json_last_error", "json_last_error_msg", "json_validate",

    // Error handling
    "trigger_error", "user_error", "set_error_handler", "restore_error_handler",
    "set_exception_handler", "restore_exception_handler", "error_reporting",
    "error_get_last", "error_clear_last", "error_log",

    // Output
    "echo", "print", "print_r", "var_dump", "var_export", "debug_print_backtrace",
    "debug_zval_dump", "ob_start", "ob_end_clean", "ob_end_flush", "ob_get_contents",
    "ob_get_clean", "ob_get_flush", "ob_get_length", "ob_get_level", "ob_get_status",
    "ob_flush", "ob_clean", "ob_implicit_flush", "ob_list_handlers",
    "output_add_rewrite_var", "output_reset_rewrite_vars",

    // Variable functions
    "compact", "extract", "define", "defined", "constant", "getenv", "putenv",
    "get_defined_vars", "get_defined_constants", "get_loaded_extensions", "get_extension_funcs",
    "get_include_path", "set_include_path", "restore_include_path",
    "ini_get", "ini_set", "ini_restore", "ini_get_all", "get_cfg_var",

    // Function handling
    "call_user_func", "call_user_func_array", "forward_static_call", "forward_static_call_array",
    "func_get_args", "func_get_arg", "func_num_args", "function_exists",
    "get_defined_functions", "create_function", "register_shutdown_function",
    "register_tick_function", "unregister_tick_function",

    // HTTP/Headers
    "header", "header_remove", "headers_list", "headers_sent", "http_response_code",
    "setcookie", "setrawcookie",

    // Session
    "session_start", "session_destroy", "session_unset", "session_regenerate_id",
    "session_id", "session_name", "session_save_path", "session_status",
    "session_cache_expire", "session_cache_limiter", "session_module_name",
    "session_set_cookie_params", "session_get_cookie_params",
    "session_encode", "session_decode", "session_write_close", "session_abort",
    "session_reset", "session_gc", "session_create_id",

    // Process control
    "exit", "die", "sleep", "usleep", "time_nanosleep", "time_sleep_until",
    "flush", "set_time_limit", "ignore_user_abort", "connection_aborted",
    "connection_status", "getmypid", "getmyuid", "getmygid", "getmyinode",
    "getlastmod", "get_current_user", "memory_get_usage", "memory_get_peak_usage",
    "memory_reset_peak_usage", "sys_get_temp_dir", "gc_collect_cycles", "gc_enabled",
    "gc_enable", "gc_disable", "gc_status", "gc_mem_caches",

    // System
    "phpinfo", "phpversion", "phpcredits", "php_uname", "php_sapi_name",
    "zend_version", "zend_thread_id", "getopt", "get_include_path",
    "php_ini_loaded_file", "php_ini_scanned_files",
    "exec", "shell_exec", "system", "passthru", "escapeshellarg", "escapeshellcmd",
    "proc_nice", "getenv", "putenv", "getmypid", "getmyuid", "get_current_user",

    // Serialization
    "serialize", "unserialize", "igbinary_serialize", "igbinary_unserialize",
    "msgpack_pack", "msgpack_unpack",

    // Cryptography
    "password_hash", "password_verify", "password_needs_rehash", "password_get_info", "password_algos",
    "hash", "hash_hmac", "hash_init", "hash_update", "hash_update_file", "hash_update_stream",
    "hash_final", "hash_copy", "hash_file", "hash_algos", "hash_hmac_algos", "hash_pbkdf2",
    "hash_equals", "hash_hkdf", "md5", "md5_file", "sha1", "sha1_file", "crc32",
    "crypt", "str_rot13",
    "openssl_encrypt", "openssl_decrypt", "openssl_cipher_iv_length", "openssl_get_cipher_methods",
    "openssl_digest", "openssl_get_md_methods", "openssl_random_pseudo_bytes",
    "openssl_sign", "openssl_verify", "openssl_seal", "openssl_open",
    "openssl_pkey_new", "openssl_pkey_get_public", "openssl_pkey_get_private",
    "openssl_pkey_get_details", "openssl_pkey_export", "openssl_pkey_export_to_file",
    "sodium_crypto_secretbox", "sodium_crypto_secretbox_open", "sodium_crypto_box",
    "sodium_crypto_box_open", "sodium_crypto_sign", "sodium_crypto_sign_open",
    "sodium_bin2hex", "sodium_hex2bin", "sodium_memzero", "sodium_randombytes_buf",

    // Encoding
    "base64_encode", "base64_decode", "urlencode", "urldecode", "rawurlencode", "rawurldecode",
    "pack", "unpack", "hex2bin", "bin2hex", "convert_cyr_string", "convert_uuencode", "convert_uudecode",

    // Compression
    "gzcompress", "gzuncompress", "gzdeflate", "gzinflate", "gzencode", "gzdecode",
    "zlib_encode", "zlib_decode", "zlib_get_coding_type",
    "bzcompress", "bzdecompress", "bzopen", "bzread", "bzwrite", "bzclose",

    // Images (GD)
    "imagecreate", "imagecreatetruecolor", "imagecreatefromjpeg", "imagecreatefrompng",
    "imagecreatefromgif", "imagecreatefromwebp", "imagecreatefromstring",
    "imagejpeg", "imagepng", "imagegif", "imagewebp", "imagedestroy",
    "imagecolorallocate", "imagecolorallocatealpha", "imagecolortransparent",
    "imagesx", "imagesy", "imagecopy", "imagecopyresampled", "imagecopyresized",
    "imagestring", "imagettftext", "imagettfbbox", "imagesetpixel", "imagegetpixel",
    "imagefilledrectangle", "imagefilledellipse", "imageline", "imagearc",
    "imagerotate", "imageflip", "imagescale", "imagecrop", "imagecropauto",
    "getimagesize", "getimagesizefromstring", "image_type_to_mime_type", "image_type_to_extension",
    "exif_read_data", "exif_thumbnail", "exif_imagetype",

    // cURL
    "curl_init", "curl_close", "curl_setopt", "curl_setopt_array", "curl_exec",
    "curl_getinfo", "curl_error", "curl_errno", "curl_reset", "curl_escape", "curl_unescape",
    "curl_multi_init", "curl_multi_add_handle", "curl_multi_remove_handle", "curl_multi_exec",
    "curl_multi_select", "curl_multi_getcontent", "curl_multi_info_read", "curl_multi_close",
    "curl_multi_setopt", "curl_multi_strerror",
    "curl_share_init", "curl_share_close", "curl_share_setopt", "curl_share_strerror",
    "curl_strerror", "curl_version", "curl_copy_handle", "curl_pause", "curl_upkeep",

    // PDO/Database
    "pdo_drivers",

    // Misc
    "assert", "assert_options", "version_compare", "extension_loaded",
    "dl", "cli_get_process_title", "cli_set_process_title",
    "debug_backtrace", "debug_print_backtrace",
    "highlight_file", "highlight_string", "show_source", "php_strip_whitespace",
    "token_get_all", "token_name",
    "get_browser", "getrusage", "getmxrr", "checkdnsrr", "dns_check_record", "gethostbyname",
    "gethostbyaddr", "gethostname", "gethostbynamel", "dns_get_record", "dns_get_mx",
    "inet_pton", "inet_ntop", "ip2long", "long2ip",
    "getprotobynumber", "getprotobyname", "getservbyname", "getservbyport",
    "socket_create", "socket_bind", "socket_listen", "socket_accept", "socket_connect",
    "socket_read", "socket_write", "socket_close", "socket_last_error", "socket_strerror",
    "usort", "uksort", "uasort",
    "array_multisort",
    "preg_filter",
    "sprintf", "sscanf", "fprintf", "fscanf",
    "array_walk", "array_walk_recursive",
    "__halt_compiler",
];

/// PHP built-in classes (comprehensive list)
pub const PHP_BUILTIN_CLASSES: &[&str] = &[
    // Core classes
    "stdClass", "self", "static", "parent",

    // Exceptions
    "Exception", "Error", "TypeError", "ArgumentCountError", "ValueError",
    "ArithmeticError", "DivisionByZeroError", "AssertionError", "ParseError",
    "CompileError", "ErrorException", "UnhandledMatchError",
    "RuntimeException", "LogicException", "InvalidArgumentException", "OutOfBoundsException",
    "OutOfRangeException", "OverflowException", "UnderflowException", "UnexpectedValueException",
    "DomainException", "RangeException", "LengthException", "BadMethodCallException",
    "BadFunctionCallException",

    // Iterators
    "Iterator", "IteratorAggregate", "OuterIterator", "RecursiveIterator", "SeekableIterator",
    "ArrayIterator", "RecursiveArrayIterator", "DirectoryIterator", "RecursiveDirectoryIterator",
    "FilesystemIterator", "GlobIterator", "RegexIterator", "RecursiveRegexIterator",
    "FilterIterator", "RecursiveFilterIterator", "CallbackFilterIterator",
    "RecursiveCallbackFilterIterator", "ParentIterator",
    "LimitIterator", "InfiniteIterator", "EmptyIterator", "IteratorIterator",
    "AppendIterator", "MultipleIterator", "NoRewindIterator", "CachingIterator",
    "RecursiveCachingIterator", "RecursiveTreeIterator",

    // Data structures
    "ArrayObject", "SplFixedArray", "SplDoublyLinkedList", "SplStack", "SplQueue",
    "SplHeap", "SplMinHeap", "SplMaxHeap", "SplPriorityQueue", "SplObjectStorage",

    // File handling
    "SplFileInfo", "SplFileObject", "SplTempFileObject",

    // DateTime
    "DateTime", "DateTimeImmutable", "DateTimeZone", "DateInterval", "DatePeriod",
    "DateTimeInterface", "DateError", "DateException", "DateInvalidOperationException",
    "DateInvalidTimeZoneException", "DateMalformedIntervalStringException",
    "DateMalformedPeriodStringException", "DateMalformedStringException",
    "DateObjectError", "DateRangeError",

    // Reflection
    "Reflector", "ReflectionException", "Reflection",
    "ReflectionClass", "ReflectionObject", "ReflectionMethod", "ReflectionProperty",
    "ReflectionFunction", "ReflectionFunctionAbstract", "ReflectionZendExtension",
    "ReflectionParameter", "ReflectionType", "ReflectionNamedType", "ReflectionUnionType",
    "ReflectionIntersectionType", "ReflectionClassConstant", "ReflectionExtension",
    "ReflectionAttribute", "ReflectionEnum", "ReflectionEnumUnitCase", "ReflectionEnumBackedCase",
    "ReflectionGenerator", "ReflectionFiber", "ReflectionReference",

    // Generators
    "Generator", "ClosedGeneratorException",

    // Closure
    "Closure",

    // Interfaces
    "Traversable", "Countable", "ArrayAccess", "Serializable", "JsonSerializable",
    "Stringable", "Throwable", "IteratorAggregate", "Iterator", "UnitEnum", "BackedEnum",

    // Attributes
    "Attribute", "ReturnTypeWillChange", "AllowDynamicProperties", "SensitiveParameter",
    "Override", "Deprecated",

    // Weak references
    "WeakReference", "WeakMap",

    // Fibers (PHP 8.1+)
    "Fiber", "FiberError",

    // Random (PHP 8.2+)
    "Random\\Randomizer", "Random\\Engine", "Random\\Engine\\Mt19937",
    "Random\\Engine\\PcgOneseq128XslRr64", "Random\\Engine\\Xoshiro256StarStar",
    "Random\\Engine\\Secure", "Random\\RandomError", "Random\\BrokenRandomEngineError",
    "Random\\RandomException", "Random\\IntervalBoundary",

    // DOM
    "DOMDocument", "DOMElement", "DOMNode", "DOMNodeList", "DOMAttr", "DOMText",
    "DOMComment", "DOMCdataSection", "DOMDocumentFragment", "DOMDocumentType",
    "DOMEntity", "DOMEntityReference", "DOMNotation", "DOMProcessingInstruction",
    "DOMException", "DOMImplementation", "DOMNamedNodeMap", "DOMCharacterData",
    "DOMXPath", "DOMNameSpaceNode", "DOMParentNode", "DOMChildNode",

    // SimpleXML
    "SimpleXMLElement", "SimpleXMLIterator",

    // XMLReader/XMLWriter
    "XMLReader", "XMLWriter",

    // libxml
    "LibXMLError",

    // PDO
    "PDO", "PDOStatement", "PDOException", "PDORow",

    // MySQLi
    "mysqli", "mysqli_stmt", "mysqli_result", "mysqli_driver", "mysqli_warning",
    "mysqli_sql_exception",

    // SQLite3
    "SQLite3", "SQLite3Stmt", "SQLite3Result", "SQLite3Exception",

    // cURL
    "CurlHandle", "CurlMultiHandle", "CurlShareHandle",

    // GD
    "GdImage", "GdFont",

    // Intl
    "Collator", "NumberFormatter", "Locale", "Normalizer", "MessageFormatter",
    "IntlDateFormatter", "IntlCalendar", "IntlGregorianCalendar", "IntlTimeZone",
    "ResourceBundle", "Spoofchecker", "Transliterator", "IntlBreakIterator",
    "IntlRuleBasedBreakIterator", "IntlCodePointBreakIterator", "IntlPartsIterator",
    "UConverter", "IntlChar", "IntlIterator", "IntlException",

    // Zip
    "ZipArchive",

    // Phar
    "Phar", "PharData", "PharFileInfo", "PharException",

    // PCRE
    "InternalIterator",

    // Sockets
    "Socket", "AddressInfo",

    // OpenSSL
    "OpenSSLCertificate", "OpenSSLCertificateSigningRequest", "OpenSSLAsymmetricKey",

    // FTP
    "FTP\\Connection",

    // LDAP
    "LDAP\\Connection", "LDAP\\Result", "LDAP\\ResultEntry",

    // PgSQL
    "PgSql\\Connection", "PgSql\\Result", "PgSql\\Lob",

    // Shmop
    "Shmop",

    // SNMP
    "SNMP", "SNMPException",

    // Soap
    "SoapClient", "SoapServer", "SoapFault", "SoapHeader", "SoapParam", "SoapVar",

    // Spl types
    "SplObserver", "SplSubject", "SplType", "SplInt", "SplFloat", "SplEnum", "SplBool", "SplString",

    // finfo
    "finfo",

    // Directory
    "Directory",

    // php_user_filter
    "php_user_filter",

    // Hashcontext
    "HashContext",

    // InflateContext/DeflateContext
    "InflateContext", "DeflateContext",
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
