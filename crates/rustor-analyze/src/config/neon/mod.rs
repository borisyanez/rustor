//! NEON format parser
//!
//! NEON is the configuration format used by PHPStan and other Nette framework tools.
//! It's similar to YAML but with some differences.
//!
//! Key features:
//! - Key-value pairs with `:` or `=`
//! - Arrays with `-` prefix (block) or `[...]` (inline)
//! - Objects with `{...}` (inline)
//! - Comments with `#`
//! - Multi-line strings
//! - Includes directive

mod lexer;
mod parser;

pub use lexer::{Token, TokenKind, Lexer};
pub use parser::{Value, NeonParser, ParseError};

/// Parse a NEON string into a Value
pub fn parse(input: &str) -> Result<Value, ParseError> {
    let mut parser = NeonParser::new(input);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_key_value() {
        let input = "level: 5";
        let result = parse(input).unwrap();
        assert!(matches!(result, Value::Object(_)));
        if let Value::Object(map) = result {
            assert_eq!(map.get("level"), Some(&Value::Integer(5)));
        }
    }

    #[test]
    fn test_parse_array() {
        let input = r#"
paths:
    - src/
    - tests/
"#;
        let result = parse(input).unwrap();
        if let Value::Object(map) = result {
            if let Some(Value::Array(arr)) = map.get("paths") {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0], Value::String("src/".to_string()));
                assert_eq!(arr[1], Value::String("tests/".to_string()));
            } else {
                panic!("Expected array for paths");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_nested_object() {
        let input = r#"
parameters:
    level: 5
    paths:
        - src/
"#;
        let result = parse(input).unwrap();
        if let Value::Object(map) = result {
            if let Some(Value::Object(params)) = map.get("parameters") {
                assert_eq!(params.get("level"), Some(&Value::Integer(5)));
            } else {
                panic!("Expected object for parameters");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_inline_array() {
        let input = "paths: [src/, tests/]";
        let result = parse(input).unwrap();
        if let Value::Object(map) = result {
            if let Some(Value::Array(arr)) = map.get("paths") {
                assert_eq!(arr.len(), 2);
            } else {
                panic!("Expected array");
            }
        }
    }

    #[test]
    fn test_parse_inline_object() {
        let input = "config: {level: 5, strict: true}";
        let result = parse(input).unwrap();
        if let Value::Object(map) = result {
            if let Some(Value::Object(config)) = map.get("config") {
                assert_eq!(config.get("level"), Some(&Value::Integer(5)));
                assert_eq!(config.get("strict"), Some(&Value::Bool(true)));
            } else {
                panic!("Expected object for config");
            }
        }
    }

    #[test]
    fn test_parse_comments() {
        let input = r#"
# This is a comment
level: 5  # inline comment
"#;
        let result = parse(input).unwrap();
        if let Value::Object(map) = result {
            assert_eq!(map.get("level"), Some(&Value::Integer(5)));
        }
    }

    #[test]
    fn test_parse_quoted_string() {
        let input = r#"message: "Hello, World!""#;
        let result = parse(input).unwrap();
        if let Value::Object(map) = result {
            assert_eq!(map.get("message"), Some(&Value::String("Hello, World!".to_string())));
        }
    }

    #[test]
    fn test_parse_includes() {
        let input = r#"
includes:
    - vendor/phpstan/phpstan-strict-rules/rules.neon
"#;
        let result = parse(input).unwrap();
        if let Value::Object(map) = result {
            assert!(map.contains_key("includes"));
        }
    }
}
