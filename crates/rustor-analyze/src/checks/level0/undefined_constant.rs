//! Check for undefined global constants (Level 0)
//!
//! Detects usage of undefined global constants like MY_CONSTANT.
//! Does NOT check class constants (see class_constant.rs).

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashSet;
use std::path::PathBuf;

/// Checks for undefined global constant usage
pub struct UndefinedConstantCheck;

impl Check for UndefinedConstantCheck {
    fn id(&self) -> &'static str {
        "constant.notFound"
    }

    fn description(&self) -> &'static str {
        "Detects usage of undefined global constants"
    }

    fn level(&self) -> u8 {
        0
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = UndefinedConstantVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            defined_constants: HashSet::new(),
            symbol_table: ctx.symbol_table,
            issues: Vec::new(),
        };

        // First pass: collect constant definitions
        visitor.collect_definitions(program);

        // Second pass: check constant usage
        visitor.check_program(program);

        visitor.issues
    }
}

struct UndefinedConstantVisitor<'s> {
    source: &'s str,
    file_path: PathBuf,
    defined_constants: HashSet<String>,
    symbol_table: Option<&'s SymbolTable>,
    issues: Vec<Issue>,
}

impl<'s> UndefinedConstantVisitor<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn get_line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// Collect all constant definitions (define() calls and const declarations)
    fn collect_definitions<'a>(&mut self, program: &Program<'a>) {
        // Add PHP built-in constants
        self.add_builtin_constants();

        // Collect user-defined constants
        for stmt in program.statements.iter() {
            self.collect_from_stmt(stmt);
        }
    }

    fn add_builtin_constants(&mut self) {
        // PHP built-in constants
        let builtins = [
            // Boolean/null literals
            "TRUE", "FALSE", "NULL",

            // PHP version and system constants
            "PHP_VERSION", "PHP_MAJOR_VERSION", "PHP_MINOR_VERSION", "PHP_RELEASE_VERSION",
            "PHP_VERSION_ID", "PHP_EXTRA_VERSION", "PHP_ZTS", "PHP_DEBUG",
            "PHP_OS", "PHP_OS_FAMILY", "PHP_SAPI", "PHP_EOL",
            "PHP_INT_MAX", "PHP_INT_MIN", "PHP_INT_SIZE",
            "PHP_FLOAT_DIG", "PHP_FLOAT_EPSILON", "PHP_FLOAT_MAX", "PHP_FLOAT_MIN",
            "PHP_MAXPATHLEN", "PHP_BINARY", "PHP_SHLIB_SUFFIX", "PHP_PREFIX",

            // Error reporting constants
            "E_ERROR", "E_WARNING", "E_PARSE", "E_NOTICE",
            "E_CORE_ERROR", "E_CORE_WARNING", "E_COMPILE_ERROR",
            "E_COMPILE_WARNING", "E_USER_ERROR", "E_USER_WARNING",
            "E_USER_NOTICE", "E_STRICT", "E_RECOVERABLE_ERROR",
            "E_DEPRECATED", "E_USER_DEPRECATED", "E_ALL",

            // Path/directory constants
            "DIRECTORY_SEPARATOR", "PATH_SEPARATOR",

            // Magic constants
            "__FILE__", "__LINE__", "__DIR__", "__FUNCTION__",
            "__CLASS__", "__METHOD__", "__NAMESPACE__", "__TRAIT__",

            // Standard streams
            "STDIN", "STDOUT", "STDERR",

            // File system constants
            "LOCK_SH", "LOCK_EX", "LOCK_UN", "LOCK_NB",
            "FILE_USE_INCLUDE_PATH", "FILE_IGNORE_NEW_LINES", "FILE_SKIP_EMPTY_LINES",
            "FILE_APPEND", "FILE_NO_DEFAULT_CONTEXT", "FILE_TEXT", "FILE_BINARY",
            "SEEK_SET", "SEEK_CUR", "SEEK_END",
            "GLOB_BRACE", "GLOB_ONLYDIR", "GLOB_MARK", "GLOB_NOSORT", "GLOB_NOCHECK",
            "GLOB_NOESCAPE", "GLOB_AVAILABLE_FLAGS", "GLOB_ERR",
            "PATHINFO_DIRNAME", "PATHINFO_BASENAME", "PATHINFO_EXTENSION", "PATHINFO_FILENAME",
            "SCANDIR_SORT_ASCENDING", "SCANDIR_SORT_DESCENDING", "SCANDIR_SORT_NONE",

            // JSON constants
            "JSON_HEX_TAG", "JSON_HEX_AMP", "JSON_HEX_APOS", "JSON_HEX_QUOT",
            "JSON_FORCE_OBJECT", "JSON_NUMERIC_CHECK", "JSON_BIGINT_AS_STRING",
            "JSON_PRETTY_PRINT", "JSON_UNESCAPED_SLASHES", "JSON_UNESCAPED_UNICODE",
            "JSON_PARTIAL_OUTPUT_ON_ERROR", "JSON_PRESERVE_ZERO_FRACTION",
            "JSON_UNESCAPED_LINE_TERMINATORS", "JSON_THROW_ON_ERROR",
            "JSON_INVALID_UTF8_IGNORE", "JSON_INVALID_UTF8_SUBSTITUTE",
            "JSON_ERROR_NONE", "JSON_ERROR_DEPTH", "JSON_ERROR_STATE_MISMATCH",
            "JSON_ERROR_CTRL_CHAR", "JSON_ERROR_SYNTAX", "JSON_ERROR_UTF8",
            "JSON_ERROR_RECURSION", "JSON_ERROR_INF_OR_NAN", "JSON_ERROR_UNSUPPORTED_TYPE",

            // MySQLi constants
            "MYSQLI_ASSOC", "MYSQLI_NUM", "MYSQLI_BOTH",
            "MYSQLI_CLIENT_COMPRESS", "MYSQLI_CLIENT_SSL", "MYSQLI_CLIENT_INTERACTIVE",
            "MYSQLI_CLIENT_IGNORE_SPACE", "MYSQLI_CLIENT_NO_SCHEMA",
            "MYSQLI_STORE_RESULT", "MYSQLI_USE_RESULT",
            "MYSQLI_AUTO_RECONNECT", "MYSQLI_OPT_CONNECT_TIMEOUT", "MYSQLI_OPT_READ_TIMEOUT",
            "MYSQLI_REPORT_OFF", "MYSQLI_REPORT_ERROR", "MYSQLI_REPORT_STRICT",
            "MYSQLI_REPORT_INDEX", "MYSQLI_REPORT_ALL",
            "MYSQLI_TYPE_DECIMAL", "MYSQLI_TYPE_TINY", "MYSQLI_TYPE_SHORT",
            "MYSQLI_TYPE_LONG", "MYSQLI_TYPE_FLOAT", "MYSQLI_TYPE_DOUBLE",
            "MYSQLI_TYPE_NULL", "MYSQLI_TYPE_TIMESTAMP", "MYSQLI_TYPE_LONGLONG",
            "MYSQLI_TYPE_INT24", "MYSQLI_TYPE_DATE", "MYSQLI_TYPE_TIME",
            "MYSQLI_TYPE_DATETIME", "MYSQLI_TYPE_YEAR", "MYSQLI_TYPE_NEWDATE",
            "MYSQLI_TYPE_VARCHAR", "MYSQLI_TYPE_BIT", "MYSQLI_TYPE_JSON",
            "MYSQLI_TYPE_NEWDECIMAL", "MYSQLI_TYPE_ENUM", "MYSQLI_TYPE_SET",
            "MYSQLI_TYPE_TINY_BLOB", "MYSQLI_TYPE_MEDIUM_BLOB", "MYSQLI_TYPE_LONG_BLOB",
            "MYSQLI_TYPE_BLOB", "MYSQLI_TYPE_VAR_STRING", "MYSQLI_TYPE_STRING",
            "MYSQLI_TYPE_CHAR", "MYSQLI_TYPE_INTERVAL", "MYSQLI_TYPE_GEOMETRY",

            // Filter constants
            "FILTER_FLAG_NONE", "FILTER_REQUIRE_SCALAR", "FILTER_REQUIRE_ARRAY",
            "FILTER_FORCE_ARRAY", "FILTER_NULL_ON_FAILURE",
            "FILTER_VALIDATE_INT", "FILTER_VALIDATE_BOOLEAN", "FILTER_VALIDATE_BOOL",
            "FILTER_VALIDATE_FLOAT", "FILTER_VALIDATE_REGEXP", "FILTER_VALIDATE_URL",
            "FILTER_VALIDATE_EMAIL", "FILTER_VALIDATE_IP", "FILTER_VALIDATE_MAC",
            "FILTER_VALIDATE_DOMAIN", "FILTER_DEFAULT", "FILTER_UNSAFE_RAW",
            "FILTER_SANITIZE_STRING", "FILTER_SANITIZE_STRIPPED", "FILTER_SANITIZE_ENCODED",
            "FILTER_SANITIZE_SPECIAL_CHARS", "FILTER_SANITIZE_FULL_SPECIAL_CHARS",
            "FILTER_SANITIZE_EMAIL", "FILTER_SANITIZE_URL", "FILTER_SANITIZE_NUMBER_INT",
            "FILTER_SANITIZE_NUMBER_FLOAT", "FILTER_SANITIZE_MAGIC_QUOTES",
            "FILTER_SANITIZE_ADD_SLASHES", "FILTER_CALLBACK",
            "FILTER_FLAG_ALLOW_OCTAL", "FILTER_FLAG_ALLOW_HEX",
            "FILTER_FLAG_STRIP_LOW", "FILTER_FLAG_STRIP_HIGH", "FILTER_FLAG_STRIP_BACKTICK",
            "FILTER_FLAG_ENCODE_LOW", "FILTER_FLAG_ENCODE_HIGH", "FILTER_FLAG_ENCODE_AMP",
            "FILTER_FLAG_NO_ENCODE_QUOTES", "FILTER_FLAG_EMPTY_STRING_NULL",
            "FILTER_FLAG_ALLOW_FRACTION", "FILTER_FLAG_ALLOW_THOUSAND",
            "FILTER_FLAG_ALLOW_SCIENTIFIC", "FILTER_FLAG_PATH_REQUIRED",
            "FILTER_FLAG_QUERY_REQUIRED", "FILTER_FLAG_SCHEME_REQUIRED",
            "FILTER_FLAG_HOST_REQUIRED", "FILTER_FLAG_HOSTNAME",
            "FILTER_FLAG_IPV4", "FILTER_FLAG_IPV6", "FILTER_FLAG_NO_RES_RANGE",
            "FILTER_FLAG_NO_PRIV_RANGE", "FILTER_FLAG_GLOBAL_RANGE",
            "INPUT_POST", "INPUT_GET", "INPUT_COOKIE", "INPUT_SERVER", "INPUT_ENV",
            "INPUT_SESSION", "INPUT_REQUEST",

            // String constants
            "STR_PAD_LEFT", "STR_PAD_RIGHT", "STR_PAD_BOTH",
            "CRYPT_SALT_LENGTH", "CRYPT_STD_DES", "CRYPT_EXT_DES",
            "CRYPT_MD5", "CRYPT_BLOWFISH", "CRYPT_SHA256", "CRYPT_SHA512",
            "CHAR_MAX", "LC_CTYPE", "LC_NUMERIC", "LC_TIME", "LC_COLLATE",
            "LC_MONETARY", "LC_ALL", "LC_MESSAGES",
            "HTML_SPECIALCHARS", "HTML_ENTITIES",
            "ENT_COMPAT", "ENT_QUOTES", "ENT_NOQUOTES", "ENT_IGNORE",
            "ENT_SUBSTITUTE", "ENT_DISALLOWED", "ENT_HTML401", "ENT_XML1",
            "ENT_XHTML", "ENT_HTML5",

            // Array constants
            "SORT_ASC", "SORT_DESC",
            "SORT_REGULAR", "SORT_NUMERIC", "SORT_STRING", "SORT_LOCALE_STRING",
            "SORT_NATURAL", "SORT_FLAG_CASE",
            "CASE_LOWER", "CASE_UPPER",
            "COUNT_NORMAL", "COUNT_RECURSIVE",
            "ARRAY_FILTER_USE_KEY", "ARRAY_FILTER_USE_BOTH",
            "EXTR_OVERWRITE", "EXTR_SKIP", "EXTR_IF_EXISTS", "EXTR_PREFIX_SAME",
            "EXTR_PREFIX_ALL", "EXTR_PREFIX_INVALID", "EXTR_PREFIX_IF_EXISTS",
            "EXTR_REFS",

            // PREG constants
            "PREG_PATTERN_ORDER", "PREG_SET_ORDER", "PREG_OFFSET_CAPTURE",
            "PREG_UNMATCHED_AS_NULL", "PREG_SPLIT_NO_EMPTY", "PREG_SPLIT_DELIM_CAPTURE",
            "PREG_SPLIT_OFFSET_CAPTURE", "PREG_GREP_INVERT", "PREG_NO_ERROR",
            "PREG_INTERNAL_ERROR", "PREG_BACKTRACK_LIMIT_ERROR",
            "PREG_RECURSION_LIMIT_ERROR", "PREG_BAD_UTF8_ERROR",
            "PREG_BAD_UTF8_OFFSET_ERROR", "PREG_JIT_STACKLIMIT_ERROR",

            // cURL constants
            "CURLOPT_AUTOREFERER", "CURLOPT_BINARYTRANSFER", "CURLOPT_BUFFERSIZE",
            "CURLOPT_CAINFO", "CURLOPT_CAPATH", "CURLOPT_CONNECTTIMEOUT",
            "CURLOPT_CONNECTTIMEOUT_MS", "CURLOPT_COOKIE", "CURLOPT_COOKIEFILE",
            "CURLOPT_COOKIEJAR", "CURLOPT_COOKIESESSION", "CURLOPT_CRLF",
            "CURLOPT_CUSTOMREQUEST", "CURLOPT_DNS_CACHE_TIMEOUT",
            "CURLOPT_DNS_USE_GLOBAL_CACHE", "CURLOPT_ENCODING", "CURLOPT_FAILONERROR",
            "CURLOPT_FILE", "CURLOPT_FILETIME", "CURLOPT_FOLLOWLOCATION",
            "CURLOPT_FORBID_REUSE", "CURLOPT_FRESH_CONNECT", "CURLOPT_FTP_USE_EPRT",
            "CURLOPT_FTP_USE_EPSV", "CURLOPT_FTPAPPEND", "CURLOPT_FTPLISTONLY",
            "CURLOPT_FTPPORT", "CURLOPT_HEADER", "CURLOPT_HEADERFUNCTION",
            "CURLOPT_HTTP200ALIASES", "CURLOPT_HTTPAUTH", "CURLOPT_HTTPGET",
            "CURLOPT_HTTPHEADER", "CURLOPT_HTTPPROXYTUNNEL", "CURLOPT_INFILE",
            "CURLOPT_INFILESIZE", "CURLOPT_INTERFACE", "CURLOPT_IPRESOLVE",
            "CURLOPT_LOW_SPEED_LIMIT", "CURLOPT_LOW_SPEED_TIME",
            "CURLOPT_MAXCONNECTS", "CURLOPT_MAXREDIRS", "CURLOPT_NETRC",
            "CURLOPT_NOBODY", "CURLOPT_NOPROGRESS", "CURLOPT_NOSIGNAL",
            "CURLOPT_PASSWORD", "CURLOPT_PORT", "CURLOPT_POST",
            "CURLOPT_POSTFIELDS", "CURLOPT_POSTQUOTE", "CURLOPT_PREQUOTE",
            "CURLOPT_PRIVATE", "CURLOPT_PROGRESSFUNCTION", "CURLOPT_PROXY",
            "CURLOPT_PROXYAUTH", "CURLOPT_PROXYPORT", "CURLOPT_PROXYTYPE",
            "CURLOPT_PROXYUSERPWD", "CURLOPT_PUT", "CURLOPT_QUOTE",
            "CURLOPT_RANDOM_FILE", "CURLOPT_RANGE", "CURLOPT_READDATA",
            "CURLOPT_READFUNCTION", "CURLOPT_REFERER", "CURLOPT_RESUME_FROM",
            "CURLOPT_RETURNTRANSFER", "CURLOPT_SHARE", "CURLOPT_SSL_CIPHER_LIST",
            "CURLOPT_SSL_VERIFYHOST", "CURLOPT_SSL_VERIFYPEER", "CURLOPT_SSLCERT",
            "CURLOPT_SSLCERTPASSWD", "CURLOPT_SSLCERTTYPE", "CURLOPT_SSLENGINE",
            "CURLOPT_SSLENGINE_DEFAULT", "CURLOPT_SSLKEY", "CURLOPT_SSLKEYPASSWD",
            "CURLOPT_SSLKEYTYPE", "CURLOPT_SSLVERSION", "CURLOPT_STDERR",
            "CURLOPT_TCP_NODELAY", "CURLOPT_TIMECONDITION", "CURLOPT_TIMEOUT",
            "CURLOPT_TIMEOUT_MS", "CURLOPT_TIMEVALUE", "CURLOPT_TRANSFERTEXT",
            "CURLOPT_UNRESTRICTED_AUTH", "CURLOPT_UPLOAD", "CURLOPT_URL",
            "CURLOPT_USERAGENT", "CURLOPT_USERNAME", "CURLOPT_USERPWD",
            "CURLOPT_VERBOSE", "CURLOPT_WRITEFUNCTION", "CURLOPT_WRITEHEADER",
            "CURLAUTH_ANY", "CURLAUTH_ANYSAFE", "CURLAUTH_BASIC",
            "CURLAUTH_DIGEST", "CURLAUTH_GSSNEGOTIATE", "CURLAUTH_NONE",
            "CURLAUTH_NTLM",
            "CURLE_OK", "CURLE_UNSUPPORTED_PROTOCOL", "CURLE_URL_MALFORMAT",
            "CURLE_COULDNT_RESOLVE_HOST", "CURLE_COULDNT_CONNECT",
            "CURLE_OPERATION_TIMEOUTED", "CURLE_SSL_CONNECT_ERROR",
            "CURLINFO_EFFECTIVE_URL", "CURLINFO_HTTP_CODE", "CURLINFO_RESPONSE_CODE",
            "CURLINFO_FILETIME", "CURLINFO_TOTAL_TIME", "CURLINFO_NAMELOOKUP_TIME",
            "CURLINFO_CONNECT_TIME", "CURLINFO_PRETRANSFER_TIME",
            "CURLINFO_SIZE_UPLOAD", "CURLINFO_SIZE_DOWNLOAD",
            "CURLINFO_SPEED_DOWNLOAD", "CURLINFO_SPEED_UPLOAD",
            "CURLINFO_HEADER_SIZE", "CURLINFO_REQUEST_SIZE",
            "CURLINFO_SSL_VERIFYRESULT", "CURLINFO_CONTENT_LENGTH_DOWNLOAD",
            "CURLINFO_CONTENT_LENGTH_UPLOAD", "CURLINFO_CONTENT_TYPE",
            "CURLINFO_REDIRECT_COUNT", "CURLINFO_REDIRECT_TIME",
            "CURLINFO_HEADER_OUT",
            "CURLM_OK", "CURLM_CALL_MULTI_PERFORM",
            "CURLMSG_DONE",
            "CURLPROXY_HTTP", "CURLPROXY_SOCKS4", "CURLPROXY_SOCKS5",
            "CURLVERSION_NOW",

            // Date/time constants
            "DATE_ATOM", "DATE_COOKIE", "DATE_ISO8601", "DATE_ISO8601_EXPANDED",
            "DATE_RFC822", "DATE_RFC850", "DATE_RFC1036", "DATE_RFC1123",
            "DATE_RFC2822", "DATE_RFC3339", "DATE_RFC3339_EXTENDED",
            "DATE_RFC7231", "DATE_RSS", "DATE_W3C",
            "SUNFUNCS_RET_TIMESTAMP", "SUNFUNCS_RET_STRING", "SUNFUNCS_RET_DOUBLE",

            // Math constants
            "M_PI", "M_E", "M_LOG2E", "M_LOG10E", "M_LN2", "M_LN10",
            "M_PI_2", "M_PI_4", "M_1_PI", "M_2_PI", "M_SQRTPI",
            "M_2_SQRTPI", "M_SQRT2", "M_SQRT3", "M_SQRT1_2", "M_LNPI",
            "M_EULER", "NAN", "INF",
            "PHP_ROUND_HALF_UP", "PHP_ROUND_HALF_DOWN",
            "PHP_ROUND_HALF_EVEN", "PHP_ROUND_HALF_ODD",

            // Socket constants
            "AF_UNIX", "AF_INET", "AF_INET6",
            "SOCK_STREAM", "SOCK_DGRAM", "SOCK_RAW", "SOCK_SEQPACKET", "SOCK_RDM",
            "SOL_SOCKET", "SOL_TCP", "SOL_UDP",
            "SO_DEBUG", "SO_REUSEADDR", "SO_REUSEPORT", "SO_KEEPALIVE",
            "SO_DONTROUTE", "SO_LINGER", "SO_BROADCAST", "SO_OOBINLINE",
            "SO_SNDBUF", "SO_RCVBUF", "SO_SNDLOWAT", "SO_RCVLOWAT",
            "SO_SNDTIMEO", "SO_RCVTIMEO", "SO_TYPE", "SO_ERROR",
            "MSG_OOB", "MSG_WAITALL", "MSG_CTRUNC", "MSG_TRUNC",
            "MSG_PEEK", "MSG_DONTROUTE", "MSG_EOR", "MSG_EOF",
            "MSG_DONTWAIT",

            // OpenSSL constants
            "OPENSSL_VERSION_TEXT", "OPENSSL_VERSION_NUMBER",
            "OPENSSL_ALGO_SHA1", "OPENSSL_ALGO_SHA256", "OPENSSL_ALGO_SHA384",
            "OPENSSL_ALGO_SHA512", "OPENSSL_ALGO_MD5", "OPENSSL_ALGO_MD4",
            "OPENSSL_ALGO_MD2", "OPENSSL_ALGO_RMD160",
            "OPENSSL_RAW_DATA", "OPENSSL_ZERO_PADDING", "OPENSSL_NO_PADDING",
            "OPENSSL_PKCS1_PADDING", "OPENSSL_SSLV23_PADDING", "OPENSSL_PKCS1_OAEP_PADDING",
            "OPENSSL_CIPHER_RC2_40", "OPENSSL_CIPHER_RC2_128", "OPENSSL_CIPHER_RC2_64",
            "OPENSSL_CIPHER_DES", "OPENSSL_CIPHER_3DES", "OPENSSL_CIPHER_AES_128_CBC",
            "OPENSSL_CIPHER_AES_192_CBC", "OPENSSL_CIPHER_AES_256_CBC",
            "OPENSSL_KEYTYPE_RSA", "OPENSSL_KEYTYPE_DSA", "OPENSSL_KEYTYPE_DH",
            "OPENSSL_KEYTYPE_EC",
            "PKCS7_DETACHED", "PKCS7_TEXT", "PKCS7_NOINTERN", "PKCS7_NOVERIFY",
            "PKCS7_NOCHAIN", "PKCS7_NOCERTS", "PKCS7_NOATTR", "PKCS7_BINARY",
            "PKCS7_NOSIGS",
            "X509_PURPOSE_SSL_CLIENT", "X509_PURPOSE_SSL_SERVER",
            "X509_PURPOSE_NS_SSL_SERVER", "X509_PURPOSE_SMIME_SIGN",
            "X509_PURPOSE_SMIME_ENCRYPT", "X509_PURPOSE_CRL_SIGN",
            "X509_PURPOSE_ANY",

            // XML/DOM constants
            "XML_ELEMENT_NODE", "XML_ATTRIBUTE_NODE", "XML_TEXT_NODE",
            "XML_CDATA_SECTION_NODE", "XML_ENTITY_REF_NODE", "XML_ENTITY_NODE",
            "XML_PI_NODE", "XML_COMMENT_NODE", "XML_DOCUMENT_NODE",
            "XML_DOCUMENT_TYPE_NODE", "XML_DOCUMENT_FRAG_NODE", "XML_NOTATION_NODE",
            "XML_HTML_DOCUMENT_NODE", "XML_DTD_NODE", "XML_ELEMENT_DECL_NODE",
            "XML_ATTRIBUTE_DECL_NODE", "XML_ENTITY_DECL_NODE", "XML_NAMESPACE_DECL_NODE",
            "XML_LOCAL_NAMESPACE", "XML_ATTRIBUTE_CDATA", "XML_ATTRIBUTE_ID",
            "XML_ATTRIBUTE_IDREF", "XML_ATTRIBUTE_IDREFS", "XML_ATTRIBUTE_ENTITY",
            "XML_ATTRIBUTE_NMTOKEN", "XML_ATTRIBUTE_NMTOKENS",
            "XML_ATTRIBUTE_ENUMERATION", "XML_ATTRIBUTE_NOTATION",
            "XML_ERROR_NONE", "XML_ERROR_NO_MEMORY", "XML_ERROR_SYNTAX",
            "XML_ERROR_NO_ELEMENTS", "XML_ERROR_INVALID_TOKEN",
            "XML_ERROR_UNCLOSED_TOKEN", "XML_ERROR_PARTIAL_CHAR",
            "XML_ERROR_TAG_MISMATCH", "XML_ERROR_DUPLICATE_ATTRIBUTE",
            "XML_ERROR_JUNK_AFTER_DOC_ELEMENT", "XML_ERROR_PARAM_ENTITY_REF",
            "XML_ERROR_UNDEFINED_ENTITY", "XML_ERROR_RECURSIVE_ENTITY_REF",
            "XML_ERROR_ASYNC_ENTITY", "XML_ERROR_BAD_CHAR_REF",
            "XML_ERROR_BINARY_ENTITY_REF", "XML_ERROR_ATTRIBUTE_EXTERNAL_ENTITY_REF",
            "XML_ERROR_MISPLACED_XML_PI", "XML_ERROR_UNKNOWN_ENCODING",
            "XML_ERROR_INCORRECT_ENCODING", "XML_ERROR_UNCLOSED_CDATA_SECTION",
            "XML_ERROR_EXTERNAL_ENTITY_HANDLING",
            "LIBXML_VERSION", "LIBXML_DOTTED_VERSION",
            "LIBXML_LOADED_VERSION", "LIBXML_NOENT", "LIBXML_DTDLOAD",
            "LIBXML_DTDATTR", "LIBXML_DTDVALID", "LIBXML_NOERROR",
            "LIBXML_NOWARNING", "LIBXML_NOBLANKS", "LIBXML_XINCLUDE",
            "LIBXML_NSCLEAN", "LIBXML_NOCDATA", "LIBXML_NONET",
            "LIBXML_PEDANTIC", "LIBXML_COMPACT", "LIBXML_NOXMLDECL",
            "LIBXML_PARSEHUGE", "LIBXML_NOEMPTYTAG", "LIBXML_SCHEMA_CREATE",
            "LIBXML_HTML_NOIMPLIED", "LIBXML_HTML_NODEFDTD", "LIBXML_BIGLINES",
            "LIBXML_ERR_NONE", "LIBXML_ERR_WARNING", "LIBXML_ERR_ERROR",
            "LIBXML_ERR_FATAL",
            "DOM_INDEX_SIZE_ERR", "DOM_DOMSTRING_SIZE_ERR", "DOM_HIERARCHY_REQUEST_ERR",
            "DOM_WRONG_DOCUMENT_ERR", "DOM_INVALID_CHARACTER_ERR",
            "DOM_NO_DATA_ALLOWED_ERR", "DOM_NO_MODIFICATION_ALLOWED_ERR",
            "DOM_NOT_FOUND_ERR", "DOM_NOT_SUPPORTED_ERR", "DOM_INUSE_ATTRIBUTE_ERR",
            "DOM_INVALID_STATE_ERR", "DOM_SYNTAX_ERR", "DOM_INVALID_MODIFICATION_ERR",
            "DOM_NAMESPACE_ERR", "DOM_INVALID_ACCESS_ERR", "DOM_VALIDATION_ERR",

            // Image constants
            "IMAGETYPE_GIF", "IMAGETYPE_JPEG", "IMAGETYPE_PNG",
            "IMAGETYPE_SWF", "IMAGETYPE_PSD", "IMAGETYPE_BMP",
            "IMAGETYPE_TIFF_II", "IMAGETYPE_TIFF_MM", "IMAGETYPE_JPC",
            "IMAGETYPE_JP2", "IMAGETYPE_JPX", "IMAGETYPE_JB2",
            "IMAGETYPE_SWC", "IMAGETYPE_IFF", "IMAGETYPE_WBMP",
            "IMAGETYPE_XBM", "IMAGETYPE_ICO", "IMAGETYPE_WEBP",
            "IMG_GIF", "IMG_JPG", "IMG_JPEG", "IMG_PNG", "IMG_WBMP",
            "IMG_XPM", "IMG_WEBP", "IMG_BMP",
            "IMG_COLOR_TILED", "IMG_COLOR_STYLED", "IMG_COLOR_BRUSHED",
            "IMG_COLOR_STYLEDBRUSHED", "IMG_COLOR_TRANSPARENT",
            "IMG_ARC_ROUNDED", "IMG_ARC_PIE", "IMG_ARC_CHORD",
            "IMG_ARC_NOFILL", "IMG_ARC_EDGED",
            "IMG_EFFECT_REPLACE", "IMG_EFFECT_ALPHABLEND", "IMG_EFFECT_NORMAL",
            "IMG_FILTER_NEGATE", "IMG_FILTER_GRAYSCALE", "IMG_FILTER_BRIGHTNESS",
            "IMG_FILTER_CONTRAST", "IMG_FILTER_COLORIZE", "IMG_FILTER_EDGEDETECT",
            "IMG_FILTER_GAUSSIAN_BLUR", "IMG_FILTER_SELECTIVE_BLUR",
            "IMG_FILTER_EMBOSS", "IMG_FILTER_MEAN_REMOVAL", "IMG_FILTER_SMOOTH",
            "IMG_FILTER_PIXELATE", "IMG_FILTER_SCATTER",
            "GD_VERSION", "GD_MAJOR_VERSION", "GD_MINOR_VERSION",
            "GD_RELEASE_VERSION", "GD_EXTRA_VERSION", "GD_BUNDLED",

            // PCNTL constants (process control)
            "SIG_IGN", "SIG_DFL", "SIG_ERR",
            "SIGHUP", "SIGINT", "SIGQUIT", "SIGILL", "SIGTRAP",
            "SIGABRT", "SIGIOT", "SIGBUS", "SIGFPE", "SIGKILL",
            "SIGUSR1", "SIGSEGV", "SIGUSR2", "SIGPIPE", "SIGALRM",
            "SIGTERM", "SIGSTKFLT", "SIGCLD", "SIGCHLD", "SIGCONT",
            "SIGSTOP", "SIGTSTP", "SIGTTIN", "SIGTTOU", "SIGURG",
            "SIGXCPU", "SIGXFSZ", "SIGVTALRM", "SIGPROF", "SIGWINCH",
            "SIGPOLL", "SIGIO", "SIGPWR", "SIGSYS", "SIGBABY",
            "WNOHANG", "WUNTRACED", "WCONTINUED",

            // Tokenizer constants
            "T_ABSTRACT", "T_AND_EQUAL", "T_ARRAY", "T_ARRAY_CAST",
            "T_AS", "T_BAD_CHARACTER", "T_BOOLEAN_AND", "T_BOOLEAN_OR",
            "T_BOOL_CAST", "T_BREAK", "T_CALLABLE", "T_CASE", "T_CATCH",
            "T_CLASS", "T_CLASS_C", "T_CLONE", "T_CLOSE_TAG", "T_COALESCE",
            "T_COALESCE_EQUAL", "T_COMMENT", "T_CONCAT_EQUAL", "T_CONST",
            "T_CONSTANT_ENCAPSED_STRING", "T_CONTINUE", "T_CURLY_OPEN",
            "T_DEC", "T_DECLARE", "T_DEFAULT", "T_DIR", "T_DIV_EQUAL",
            "T_DNUMBER", "T_DOC_COMMENT", "T_DO", "T_DOLLAR_OPEN_CURLY_BRACES",
            "T_DOUBLE_ARROW", "T_DOUBLE_CAST", "T_DOUBLE_COLON", "T_ECHO",
            "T_ELLIPSIS", "T_ELSE", "T_ELSEIF", "T_EMPTY", "T_ENCAPSED_AND_WHITESPACE",
            "T_ENDDECLARE", "T_ENDFOR", "T_ENDFOREACH", "T_ENDIF",
            "T_ENDSWITCH", "T_ENDWHILE", "T_END_HEREDOC", "T_EVAL",
            "T_EXIT", "T_EXTENDS", "T_FILE", "T_FINAL", "T_FINALLY",
            "T_FN", "T_FOR", "T_FOREACH", "T_FUNCTION", "T_FUNC_C",
            "T_GLOBAL", "T_GOTO", "T_HALT_COMPILER", "T_IF", "T_IMPLEMENTS",
            "T_INC", "T_INCLUDE", "T_INCLUDE_ONCE", "T_INLINE_HTML",
            "T_INSTANCEOF", "T_INSTEADOF", "T_INTERFACE", "T_INT_CAST",
            "T_ISSET", "T_IS_EQUAL", "T_IS_GREATER_OR_EQUAL", "T_IS_IDENTICAL",
            "T_IS_NOT_EQUAL", "T_IS_NOT_IDENTICAL", "T_IS_SMALLER_OR_EQUAL",
            "T_LINE", "T_LIST", "T_LNUMBER", "T_LOGICAL_AND", "T_LOGICAL_OR",
            "T_LOGICAL_XOR", "T_MATCH", "T_METHOD_C", "T_MINUS_EQUAL",
            "T_MOD_EQUAL", "T_MUL_EQUAL", "T_NAMESPACE", "T_NAME_FULLY_QUALIFIED",
            "T_NAME_QUALIFIED", "T_NAME_RELATIVE", "T_NEW", "T_NS_C",
            "T_NS_SEPARATOR", "T_NUM_STRING", "T_OBJECT_CAST", "T_OBJECT_OPERATOR",
            "T_NULLSAFE_OBJECT_OPERATOR", "T_OPEN_TAG", "T_OPEN_TAG_WITH_ECHO",
            "T_OR_EQUAL", "T_PAAMAYIM_NEKUDOTAYIM", "T_PLUS_EQUAL", "T_POW",
            "T_POW_EQUAL", "T_PRINT", "T_PRIVATE", "T_PROTECTED", "T_PUBLIC",
            "T_REQUIRE", "T_REQUIRE_ONCE", "T_RETURN", "T_SL", "T_SL_EQUAL",
            "T_SPACESHIP", "T_SR", "T_SR_EQUAL", "T_START_HEREDOC",
            "T_STATIC", "T_STRING", "T_STRING_CAST", "T_STRING_VARNAME",
            "T_SWITCH", "T_THROW", "T_TRAIT", "T_TRAIT_C", "T_TRY",
            "T_UNSET", "T_UNSET_CAST", "T_USE", "T_VAR", "T_VARIABLE",
            "T_WHILE", "T_WHITESPACE", "T_XOR_EQUAL", "T_YIELD",
            "T_YIELD_FROM", "T_ATTRIBUTE", "T_ENUM", "T_READONLY",

            // Intl constants
            "INTL_MAX_LOCALE_LEN", "IDNA_DEFAULT", "IDNA_ALLOW_UNASSIGNED",
            "IDNA_USE_STD3_RULES", "IDNA_CHECK_BIDI", "IDNA_CHECK_CONTEXTJ",
            "IDNA_NONTRANSITIONAL_TO_ASCII", "IDNA_NONTRANSITIONAL_TO_UNICODE",
            "INTL_IDNA_VARIANT_2003", "INTL_IDNA_VARIANT_UTS46",
            "IDNA_ERROR_EMPTY_LABEL", "IDNA_ERROR_LABEL_TOO_LONG",
            "IDNA_ERROR_DOMAIN_NAME_TOO_LONG", "IDNA_ERROR_LEADING_HYPHEN",
            "IDNA_ERROR_TRAILING_HYPHEN", "IDNA_ERROR_HYPHEN_3_4",
            "IDNA_ERROR_LEADING_COMBINING_MARK", "IDNA_ERROR_DISALLOWED",
            "IDNA_ERROR_PUNYCODE", "IDNA_ERROR_LABEL_HAS_DOT",
            "IDNA_ERROR_INVALID_ACE_LABEL", "IDNA_ERROR_BIDI",
            "IDNA_ERROR_CONTEXTJ",

            // PDO constants
            "PDO_FETCH_ASSOC", "PDO_FETCH_NUM", "PDO_FETCH_BOTH",
            "PDO_FETCH_OBJ", "PDO_FETCH_CLASS",
            "PDO_ATTR_ERRMODE", "PDO_ERRMODE_EXCEPTION",
            "PDO_ATTR_DEFAULT_FETCH_MODE",
        ];

        for constant in &builtins {
            self.defined_constants.insert(constant.to_string());
            // Also add lowercase version for case-insensitive matching
            self.defined_constants.insert(constant.to_lowercase());
        }
    }

    fn collect_from_stmt<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            // Recurse into namespaces and blocks
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_stmt(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_stmt(inner);
                    }
                }
            },
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.collect_from_stmt(inner);
                }
            }
            // Check for define() calls in expression statements
            Statement::Expression(expr_stmt) => {
                self.collect_from_expression(&expr_stmt.expression);
            }
            // const CONSTANT_NAME = value;
            Statement::Constant(const_def) => {
                for entry in const_def.items.iter() {
                    let name = self.get_span_text(&entry.name.span);
                    self.defined_constants.insert(name.to_string());
                }
            }
            _ => {}
        }
    }

    fn collect_from_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // define('CONSTANT_NAME', value);
            Expression::Call(Call::Function(call)) => {
                // Check if this is a define() call
                if let Expression::Identifier(ident) = &*call.function {
                    let func_name = self.get_span_text(&ident.span());
                    if func_name.eq_ignore_ascii_case("define") {
                        // First argument should be the constant name (string literal)
                        if let Some(first_arg) = call.argument_list.arguments.first() {
                            if let Expression::Literal(Literal::String(s)) = first_arg.value() {
                                let const_name = self.get_span_text(&s.span());
                                // Remove quotes
                                let const_name = const_name.trim_matches(|c| c == '"' || c == '\'');
                                self.defined_constants.insert(const_name.to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Check all constant usage in the program
    fn check_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.check_statement(stmt);
        }
    }

    fn check_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.check_expression(&expr_stmt.expression);
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.check_expression(value);
                }
            }
            Statement::If(if_stmt) => {
                self.check_expression(&if_stmt.condition);
                self.check_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.check_expression(&while_stmt.condition);
                self.check_while_body(&while_stmt.body);
            }
            Statement::For(for_stmt) => {
                for init in for_stmt.initializations.iter() {
                    self.check_expression(init);
                }
                for cond in for_stmt.conditions.iter() {
                    self.check_expression(cond);
                }
                for inc in for_stmt.increments.iter() {
                    self.check_expression(inc);
                }
                self.check_for_body(&for_stmt.body);
            }
            Statement::Foreach(foreach) => {
                self.check_expression(&foreach.expression);
                self.check_foreach_body(&foreach.body);
            }
            Statement::Switch(switch) => {
                self.check_expression(&switch.expression);
                match &switch.body {
                    SwitchBody::BraceDelimited(body) => {
                        for case in body.cases.iter() {
                            match case {
                                SwitchCase::Expression(c) => {
                                    self.check_expression(&c.expression);
                                    for stmt in c.statements.iter() {
                                        self.check_statement(stmt);
                                    }
                                }
                                SwitchCase::Default(d) => {
                                    for stmt in d.statements.iter() {
                                        self.check_statement(stmt);
                                    }
                                }
                            }
                        }
                    }
                    SwitchBody::ColonDelimited(body) => {
                        for case in body.cases.iter() {
                            match case {
                                SwitchCase::Expression(c) => {
                                    self.check_expression(&c.expression);
                                    for stmt in c.statements.iter() {
                                        self.check_statement(stmt);
                                    }
                                }
                                SwitchCase::Default(d) => {
                                    for stmt in d.statements.iter() {
                                        self.check_statement(stmt);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Statement::Echo(echo) => {
                for value in echo.values.iter() {
                    self.check_expression(value);
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.check_statement(inner);
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.check_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.check_statement(inner);
                    }
                }
            },
            Statement::Class(class) => {
                for member in class.members.iter() {
                    match member {
                        ClassLikeMember::Method(method) => {
                            if let MethodBody::Concrete(body) = &method.body {
                                for stmt in body.statements.iter() {
                                    self.check_statement(stmt);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::Function(func) => {
                for stmt in func.body.statements.iter() {
                    self.check_statement(stmt);
                }
            }
            _ => {}
        }
    }

    fn check_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // This is a constant fetch - check if it's defined
            Expression::ConstantAccess(const_access) => {
                // Get the constant name - use the span directly
                let name = self.get_span_text(&const_access.name.span());

                // Skip special keywords
                if matches!(name.to_lowercase().as_str(),
                           "true" | "false" | "null") {
                    return;
                }

                // Check if constant is defined (case-insensitive for PHP constants)
                let is_defined = self.defined_constants.contains(name) ||
                    self.defined_constants.contains(&name.to_lowercase()) ||
                    // Also check the symbol table from autoload scanning
                    self.symbol_table.map_or(false, |st| st.get_constant(name).is_some());

                if !is_defined {
                    let (line, col) = self.get_line_col(const_access.name.span().start.offset as usize);
                    self.issues.push(
                        Issue::error(
                            "constant.notFound",
                            format!("Constant {} not found.", name),
                            self.file_path.clone(),
                            line,
                            col,
                        )
                        .with_identifier("constant.notFound"),
                    );
                }
            }
            Expression::Identifier(ident) => {
                let name = self.get_span_text(&ident.span());
                // Identifiers in other contexts - skip for now
            }
            // Recurse into complex expressions
            Expression::Binary(binary) => {
                self.check_expression(&binary.lhs);
                self.check_expression(&binary.rhs);
            }
            Expression::UnaryPrefix(prefix) => {
                self.check_expression(&prefix.operand);
            }
            Expression::UnaryPostfix(postfix) => {
                self.check_expression(&postfix.operand);
            }
            Expression::Conditional(cond) => {
                self.check_expression(&cond.condition);
                if let Some(then) = &cond.then {
                    self.check_expression(then);
                }
                self.check_expression(&cond.r#else);
            }
            Expression::Assignment(assign) => {
                self.check_expression(&assign.lhs);
                self.check_expression(&assign.rhs);
            }
            Expression::Array(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.check_expression(&kv.key);
                            self.check_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.check_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.check_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::LegacyArray(array) => {
                for element in array.elements.iter() {
                    match element {
                        ArrayElement::KeyValue(kv) => {
                            self.check_expression(&kv.key);
                            self.check_expression(&kv.value);
                        }
                        ArrayElement::Value(val) => {
                            self.check_expression(&val.value);
                        }
                        ArrayElement::Variadic(var) => {
                            self.check_expression(&var.value);
                        }
                        ArrayElement::Missing(_) => {}
                    }
                }
            }
            Expression::Call(call) => {
                match call {
                    Call::Function(func_call) => {
                        // Don't check the function name itself as a constant
                        // Only check the arguments
                        for arg in func_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                    Call::Method(method_call) => {
                        self.check_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                    Call::NullSafeMethod(method_call) => {
                        self.check_expression(&method_call.object);
                        for arg in method_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                    Call::StaticMethod(static_call) => {
                        for arg in static_call.argument_list.arguments.iter() {
                            self.check_expression(arg.value());
                        }
                    }
                }
            }
            Expression::Parenthesized(p) => {
                self.check_expression(&p.expression);
            }
            Expression::ArrayAccess(arr) => {
                self.check_expression(&arr.array);
                self.check_expression(&arr.index);
            }
            Expression::Construct(_) => {
            }
            other => {
            }
        }
    }

    fn check_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.check_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                    self.check_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.check_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
                for else_if in block.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                    for stmt in else_if.statements.iter() {
                        self.check_statement(stmt);
                    }
                }
                if let Some(else_clause) = &block.else_clause {
                    for stmt in else_clause.statements.iter() {
                        self.check_statement(stmt);
                    }
                }
            }
        }
    }

    fn check_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => self.check_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_for_body<'a>(&mut self, body: &ForBody<'a>) {
        match body {
            ForBody::Statement(stmt) => self.check_statement(stmt),
            ForBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }

    fn check_foreach_body<'a>(&mut self, body: &ForeachBody<'a>) {
        match body {
            ForeachBody::Statement(stmt) => self.check_statement(stmt),
            ForeachBody::ColonDelimited(block) => {
                for stmt in block.statements.iter() {
                    self.check_statement(stmt);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undefined_constant_check_level() {
        let check = UndefinedConstantCheck;
        assert_eq!(check.level(), 0);
    }
}
