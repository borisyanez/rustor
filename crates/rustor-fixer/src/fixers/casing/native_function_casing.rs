//! Fix native function casing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Ensures native PHP functions are lowercase
pub struct NativeFunctionCasingFixer;

// Common PHP native functions (subset - most commonly used)
const NATIVE_FUNCTIONS: &[&str] = &[
    "abs", "array", "array_chunk", "array_column", "array_combine", "array_count_values",
    "array_diff", "array_diff_assoc", "array_diff_key", "array_fill", "array_fill_keys",
    "array_filter", "array_flip", "array_intersect", "array_key_exists", "array_key_first",
    "array_key_last", "array_keys", "array_map", "array_merge", "array_merge_recursive",
    "array_multisort", "array_pad", "array_pop", "array_push", "array_rand", "array_reduce",
    "array_replace", "array_reverse", "array_search", "array_shift", "array_slice",
    "array_splice", "array_sum", "array_unique", "array_unshift", "array_values", "array_walk",
    "arsort", "asort", "base64_decode", "base64_encode", "basename", "bin2hex", "call_user_func",
    "call_user_func_array", "ceil", "chdir", "checkdate", "chmod", "chown", "chr", "chunk_split",
    "class_exists", "clearstatcache", "clone", "closedir", "compact", "copy", "cos", "count",
    "crc32", "crypt", "curl_close", "curl_error", "curl_exec", "curl_init", "curl_setopt",
    "current", "date", "debug_backtrace", "decbin", "dechex", "decoct", "define", "defined",
    "die", "dirname", "disk_free_space", "disk_total_space", "echo", "empty", "end", "error_log",
    "error_reporting", "escapeshellarg", "escapeshellcmd", "eval", "exec", "exit", "exp",
    "explode", "extension_loaded", "extract", "fclose", "feof", "fflush", "fgetc", "fgetcsv",
    "fgets", "file", "file_exists", "file_get_contents", "file_put_contents", "fileatime",
    "filectime", "filegroup", "fileinode", "filemtime", "fileowner", "fileperms", "filesize",
    "filetype", "floatval", "flock", "floor", "flush", "fopen", "fpassthru", "fprintf", "fputcsv",
    "fputs", "fread", "fscanf", "fseek", "fsockopen", "fstat", "ftell", "ftruncate", "fwrite",
    "get_called_class", "get_class", "get_class_methods", "get_class_vars", "get_defined_constants",
    "get_defined_functions", "get_defined_vars", "get_include_path", "get_object_vars",
    "get_parent_class", "getdate", "getenv", "gethostbyaddr", "gethostbyname", "getmypid",
    "getmyuid", "getopt", "getrandmax", "gettype", "glob", "gmdate", "gmmktime", "gmstrftime",
    "header", "headers_list", "headers_sent", "hex2bin", "highlight_file", "highlight_string",
    "htmlentities", "htmlspecialchars", "htmlspecialchars_decode", "http_build_query",
    "http_response_code", "idate", "ignore_user_abort", "implode", "in_array", "include",
    "include_once", "ini_get", "ini_restore", "ini_set", "interface_exists", "intval",
    "is_a", "is_array", "is_bool", "is_callable", "is_dir", "is_double", "is_executable",
    "is_file", "is_finite", "is_float", "is_infinite", "is_int", "is_integer", "is_link",
    "is_long", "is_nan", "is_null", "is_numeric", "is_object", "is_readable", "is_real",
    "is_resource", "is_scalar", "is_string", "is_subclass_of", "is_uploaded_file", "is_writable",
    "is_writeable", "isset", "join", "json_decode", "json_encode", "json_last_error",
    "json_last_error_msg", "key", "key_exists", "ksort", "lcfirst", "levenshtein", "link",
    "list", "localeconv", "localtime", "log", "log10", "ltrim", "mail", "max", "mb_check_encoding",
    "mb_convert_case", "mb_convert_encoding", "mb_detect_encoding", "mb_internal_encoding",
    "mb_strlen", "mb_strpos", "mb_strtolower", "mb_strtoupper", "mb_substr", "md5", "md5_file",
    "memory_get_peak_usage", "memory_get_usage", "method_exists", "microtime", "min", "mkdir",
    "mktime", "move_uploaded_file", "mt_rand", "mt_srand", "natcasesort", "natsort", "next",
    "nl2br", "number_format", "ob_clean", "ob_end_clean", "ob_end_flush", "ob_flush",
    "ob_get_clean", "ob_get_contents", "ob_get_flush", "ob_get_length", "ob_get_level",
    "ob_implicit_flush", "ob_start", "octdec", "opendir", "ord", "pack", "parse_ini_file",
    "parse_ini_string", "parse_str", "parse_url", "passthru", "pathinfo", "pclose", "pfsockopen",
    "php_ini_loaded_file", "php_sapi_name", "php_uname", "phpinfo", "phpversion", "pi", "popen",
    "pos", "pow", "preg_filter", "preg_grep", "preg_match", "preg_match_all", "preg_quote",
    "preg_replace", "preg_replace_callback", "preg_split", "prev", "print", "print_r", "printf",
    "property_exists", "putenv", "quoted_printable_decode", "quoted_printable_encode", "quotemeta",
    "rad2deg", "rand", "random_bytes", "random_int", "range", "rawurldecode", "rawurlencode",
    "read", "readdir", "readfile", "readline", "realpath", "register_shutdown_function",
    "rename", "require", "require_once", "reset", "restore_error_handler",
    "restore_exception_handler", "rewind", "rewinddir", "rmdir", "round", "rsort", "rtrim",
    "scandir", "serialize", "session_cache_expire", "session_cache_limiter", "session_decode",
    "session_destroy", "session_encode", "session_get_cookie_params", "session_id", "session_name",
    "session_regenerate_id", "session_save_path", "session_set_cookie_params",
    "session_set_save_handler", "session_start", "session_status", "session_unset",
    "session_write_close", "set_error_handler", "set_exception_handler", "set_include_path",
    "set_time_limit", "setcookie", "setlocale", "setrawcookie", "settype", "sha1", "sha1_file",
    "shell_exec", "show_source", "shuffle", "similar_text", "sin", "sizeof", "sleep", "sort",
    "soundex", "sprintf", "sqrt", "srand", "sscanf", "stat", "str_contains", "str_ends_with",
    "str_getcsv", "str_ireplace", "str_pad", "str_repeat", "str_replace", "str_rot13",
    "str_shuffle", "str_split", "str_starts_with", "str_word_count", "strcasecmp", "strchr",
    "strcmp", "strcoll", "strcspn", "stream_context_create", "stream_context_get_options",
    "stream_context_set_option", "stream_copy_to_stream", "stream_filter_append",
    "stream_filter_prepend", "stream_filter_remove", "stream_get_contents", "stream_get_filters",
    "stream_get_line", "stream_get_meta_data", "stream_get_transports", "stream_get_wrappers",
    "stream_is_local", "stream_select", "stream_set_blocking", "stream_set_chunk_size",
    "stream_set_read_buffer", "stream_set_timeout", "stream_set_write_buffer",
    "stream_socket_accept", "stream_socket_client", "stream_socket_enable_crypto",
    "stream_socket_get_name", "stream_socket_pair", "stream_socket_recvfrom",
    "stream_socket_sendto", "stream_socket_server", "stream_socket_shutdown",
    "stream_supports_lock", "stream_wrapper_register", "stream_wrapper_restore",
    "stream_wrapper_unregister", "strftime", "strip_tags", "stripcslashes", "stripos",
    "stripslashes", "stristr", "strlen", "strnatcasecmp", "strnatcmp", "strncasecmp", "strncmp",
    "strpbrk", "strpos", "strptime", "strrchr", "strrev", "strripos", "strrpos", "strspn",
    "strstr", "strtok", "strtolower", "strtotime", "strtoupper", "strtr", "strval", "substr",
    "substr_compare", "substr_count", "substr_replace", "sys_get_temp_dir", "system", "tan",
    "tempnam", "time", "timezone_abbreviations_list", "timezone_identifiers_list",
    "timezone_location_get", "timezone_name_from_abbr", "timezone_name_get", "timezone_offset_get",
    "timezone_open", "timezone_transitions_get", "timezone_version_get", "tmpfile", "touch",
    "trait_exists", "trigger_error", "trim", "uasort", "ucfirst", "ucwords", "uksort", "umask",
    "uniqid", "unlink", "unpack", "unserialize", "urldecode", "urlencode", "usleep", "usort",
    "utf8_decode", "utf8_encode", "var_dump", "var_export", "version_compare", "vfprintf",
    "vprintf", "vsprintf", "wordwrap", "write",
];

impl Fixer for NativeFunctionCasingFixer {
    fn name(&self) -> &'static str {
        "native_function_casing"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "native_function_casing"
    }

    fn description(&self) -> &'static str {
        "Ensure native PHP functions are lowercase"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match function calls that might be native functions with wrong casing
        let func_re = Regex::new(r"\b([A-Z][A-Za-z0-9_]*)\s*\(").unwrap();

        for cap in func_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let func_name = cap.get(1).unwrap();
            let func_str = func_name.as_str();
            let func_lower = func_str.to_lowercase();

            // Check if it's a native function
            if !NATIVE_FUNCTIONS.contains(&func_lower.as_str()) {
                continue;
            }

            // Already lowercase
            if func_str == func_lower {
                continue;
            }

            if is_in_string(&source[..full_match.start()]) {
                continue;
            }

            // Check it's not a method call (preceded by -> or ::)
            let before = &source[..full_match.start()];
            let trimmed = before.trim_end();
            if trimmed.ends_with("->") || trimmed.ends_with("::") {
                continue;
            }

            // Check it's not a class instantiation
            if trimmed.ends_with("new") {
                continue;
            }

            edits.push(edit_with_rule(
                func_name.start(),
                func_name.end(),
                func_lower,
                format!("Native function {} should be lowercase", func_str),
                "native_function_casing",
            ));
        }

        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for c in before.chars() {
        if c == '\'' && prev_char != '\\' && !in_double_quote {
            in_single_quote = !in_single_quote;
        }
        if c == '"' && prev_char != '\\' && !in_single_quote {
            in_double_quote = !in_double_quote;
        }
        prev_char = c;
    }

    in_single_quote || in_double_quote
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        NativeFunctionCasingFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_correct_unchanged() {
        let source = "<?php\nstrlen($a);\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_uppercase_function() {
        let source = "<?php\nSTRLEN($a);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "strlen");
    }

    #[test]
    fn test_mixed_case() {
        let source = "<?php\nStrLen($a);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "strlen");
    }

    #[test]
    fn test_array_functions() {
        let source = "<?php\nARRAY_MAP($fn, $arr);\nArray_Filter($arr);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_skip_method_call() {
        let source = "<?php\n$obj->Strlen();\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_static_call() {
        let source = "<?php\nFoo::Strlen();\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_non_native() {
        let source = "<?php\nMyCustomFunction();\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_skip_in_string() {
        let source = "<?php\n$a = 'STRLEN';\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_common_functions() {
        let source = "<?php\nCOUNT($a);\nPRINT_R($a);\nJSON_ENCODE($a);\n";
        let edits = check(source);

        assert_eq!(edits.len(), 3);
    }
}
