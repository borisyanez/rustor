//! NEON parser - parses tokens into a Value tree

use super::lexer::{Lexer, Token, TokenKind};
use std::collections::HashMap;
use thiserror::Error;

/// NEON value types
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(obj) => obj.get(key),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token at line {line}, column {column}: expected {expected}, got {got:?}")]
    UnexpectedToken {
        line: usize,
        column: usize,
        expected: String,
        got: TokenKind,
    },
    #[error("Unexpected end of file")]
    UnexpectedEof,
    #[error("Invalid indentation at line {line}")]
    InvalidIndentation { line: usize },
}

pub struct NeonParser<'a> {
    lexer: Lexer<'a>,
    tokens: Vec<Token>,
    position: usize,
}

impl<'a> NeonParser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        Self {
            lexer,
            tokens,
            position: 0,
        }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_newlines(&mut self) {
        while let Some(token) = self.current() {
            if matches!(token.kind, TokenKind::Newline) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Consume optional PHPStan (?) suffix after identifiers
    /// This is PHPStan's notation for "report if not matched"
    fn consume_optional_question_suffix(&mut self) {
        // Check for pattern: LeftParen, Identifier("?"), RightParen
        if let Some(token) = self.current() {
            if matches!(token.kind, TokenKind::LeftParen) {
                // Save position in case we need to backtrack
                let saved_pos = self.position;
                self.advance(); // consume (

                let is_question = self.current()
                    .map(|t| matches!(&t.kind, TokenKind::Identifier(s) if s == "?"))
                    .unwrap_or(false);

                if is_question {
                    self.advance(); // consume ?

                    if self.current().map(|t| matches!(t.kind, TokenKind::RightParen)).unwrap_or(false) {
                        self.advance(); // consume )
                        // Successfully consumed (?)
                        return;
                    }
                }

                // Not a (?) pattern, restore position
                self.position = saved_pos;
            }
        }
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.current().map(|t| &t.kind)
    }

    pub fn parse(&mut self) -> Result<Value, ParseError> {
        self.skip_newlines();
        self.parse_document(0)
    }

    fn parse_document(&mut self, base_indent: usize) -> Result<Value, ParseError> {
        let mut result = HashMap::new();
        let mut has_parsed_any_key = false;

        loop {
            self.skip_newlines();

            let Some(token) = self.current() else {
                break;
            };

            // Check indentation
            let current_indent = match &token.kind {
                TokenKind::Indent(n) => {
                    let indent = *n;
                    self.advance();
                    indent
                }
                TokenKind::Eof => break,
                TokenKind::Newline => {
                    self.advance();
                    continue;
                }
                _ => 0,
            };

            // If we're back to a lower indentation level, we're done with this block
            if current_indent < base_indent {
                break;
            }

            self.skip_newlines();

            let Some(token) = self.current() else {
                break;
            };

            // Check for array item - BUT only if we haven't parsed any keys yet
            // Once we start parsing keys, we're in object mode and shouldn't switch to array mode
            if matches!(token.kind, TokenKind::Dash) && !has_parsed_any_key {
                // This is an array at the top level - parse as array
                let array = self.parse_block_array(current_indent)?;
                return Ok(Value::Array(array));
            }

            // Parse key
            let key = match &token.kind {
                TokenKind::Identifier(s) => s.clone(),
                TokenKind::String(s) => s.clone(),
                TokenKind::Eof => break,
                TokenKind::Newline => {
                    self.advance();
                    continue;
                }
                _ => break,
            };
            self.advance();
            has_parsed_any_key = true;

            // Expect colon or equals
            let Some(token) = self.current() else {
                result.insert(key, Value::Null);
                break;
            };

            if !matches!(token.kind, TokenKind::Colon | TokenKind::Equals) {
                // Key without value
                result.insert(key, Value::Bool(true));
                continue;
            }
            self.advance();

            // Parse value
            let value = self.parse_value(current_indent)?;
            result.insert(key, value);
        }

        Ok(Value::Object(result))
    }

    fn parse_value(&mut self, base_indent: usize) -> Result<Value, ParseError> {
        // Check for newline followed by indented block BEFORE skipping newlines
        if let Some(token) = self.current() {
            if matches!(token.kind, TokenKind::Newline) {
                self.advance();
                return self.parse_indented_value(base_indent);
            }
        }

        // Skip any whitespace/newlines before the value
        self.skip_newlines();

        let Some(token) = self.current() else {
            return Ok(Value::Null);
        };

        match &token.kind {
            TokenKind::Null => {
                self.advance();
                Ok(Value::Null)
            }
            TokenKind::Bool(b) => {
                let value = *b;
                self.advance();
                Ok(Value::Bool(value))
            }
            TokenKind::Integer(n) => {
                let value = *n;
                self.advance();
                Ok(Value::Integer(value))
            }
            TokenKind::Float(f) => {
                let value = *f;
                self.advance();
                Ok(Value::Float(value))
            }
            TokenKind::String(s) => {
                let value = s.clone();
                self.advance();
                Ok(Value::String(value))
            }
            TokenKind::Identifier(s) => {
                let value = s.clone();
                self.advance();
                // Handle PHPStan's (?) suffix - consume it if present
                self.consume_optional_question_suffix();
                Ok(Value::String(value))
            }
            TokenKind::LeftBracket => self.parse_inline_array(),
            TokenKind::LeftBrace => self.parse_inline_object(),
            TokenKind::Newline => {
                self.advance();
                self.parse_indented_value(base_indent)
            }
            _ => Ok(Value::Null),
        }
    }

    fn parse_indented_value(&mut self, base_indent: usize) -> Result<Value, ParseError> {
        self.skip_newlines();

        let Some(token) = self.current() else {
            return Ok(Value::Null);
        };

        let current_indent = match &token.kind {
            TokenKind::Indent(n) => *n,
            _ => 0,
        };

        if current_indent <= base_indent {
            return Ok(Value::Null);
        }

        // Peek ahead to see if this is an array or object
        let saved_pos = self.position;
        if matches!(token.kind, TokenKind::Indent(_)) {
            self.advance();
        }

        let is_dash = self.current().map(|t| matches!(t.kind, TokenKind::Dash)).unwrap_or(false);

        self.position = saved_pos;

        if is_dash {
            // It's a block array
            self.advance(); // consume indent
            self.parse_block_array(current_indent).map(Value::Array)
        } else {
            // It's a nested object
            self.parse_document(current_indent)
        }
    }

    fn parse_block_array(&mut self, base_indent: usize) -> Result<Vec<Value>, ParseError> {
        let mut result = Vec::new();

        loop {
            self.skip_newlines();

            let Some(token) = self.current() else {
                // Debug: log array size when EOF
                if result.len() > 600 && result.len() < 710 {
                    eprintln!("DEBUG parse_block_array: EOF at {} items", result.len());
                }
                break;
            };

            // Check indentation
            let current_indent = match &token.kind {
                TokenKind::Indent(n) => {
                    if *n < base_indent {
                        // Debug: log when breaking due to lower indent
                        if result.len() > 600 && result.len() < 710 {
                            eprintln!("DEBUG parse_block_array: Lower indent at {} items (base={}, current={})",
                                result.len(), base_indent, *n);
                        }
                        break;
                    }
                    let indent = *n;
                    self.advance();
                    indent
                }
                TokenKind::Dash => base_indent, // First item might not have indent token
                TokenKind::Eof => {
                    if result.len() > 600 && result.len() < 710 {
                        eprintln!("DEBUG parse_block_array: EOF token at {} items", result.len());
                    }
                    break;
                }
                TokenKind::Newline => {
                    self.advance();
                    continue;
                }
                _ => {
                    // Debug: log unexpected token
                    if result.len() > 600 && result.len() < 710 {
                        eprintln!("DEBUG parse_block_array: Unexpected token {:?} at {} items, line {}, col {}",
                            token.kind, result.len(), token.line, token.column);
                    }
                    break;
                }
            };

            if current_indent < base_indent {
                if result.len() > 600 && result.len() < 710 {
                    eprintln!("DEBUG parse_block_array: indent check failed at {} items", result.len());
                }
                break;
            }

            let Some(token) = self.current() else {
                if result.len() > 600 && result.len() < 710 {
                    eprintln!("DEBUG parse_block_array: No token after indent at {} items", result.len());
                }
                break;
            };

            if !matches!(token.kind, TokenKind::Dash) {
                if result.len() > 600 && result.len() < 710 {
                    eprintln!("DEBUG parse_block_array: Expected dash, got {:?} at {} items, line {}, col {}",
                        token.kind, result.len(), token.line, token.column);
                }
                break;
            }
            self.advance();

            // Parse array item value
            let value = self.parse_value(current_indent)?;
            result.push(value);
        }

        Ok(result)
    }

    fn parse_inline_array(&mut self) -> Result<Value, ParseError> {
        self.advance(); // consume [

        let mut result = Vec::new();

        loop {
            self.skip_newlines();

            let Some(token) = self.current() else {
                break;
            };

            if matches!(token.kind, TokenKind::RightBracket) {
                self.advance();
                break;
            }

            let value = self.parse_value(0)?;
            result.push(value);

            // Skip comma
            if let Some(token) = self.current() {
                if matches!(token.kind, TokenKind::Comma) {
                    self.advance();
                }
            }
        }

        Ok(Value::Array(result))
    }

    fn parse_inline_object(&mut self) -> Result<Value, ParseError> {
        self.advance(); // consume {

        let mut result = HashMap::new();

        loop {
            self.skip_newlines();

            let Some(token) = self.current() else {
                break;
            };

            if matches!(token.kind, TokenKind::RightBrace) {
                self.advance();
                break;
            }

            // Parse key
            let key = match &token.kind {
                TokenKind::Identifier(s) => s.clone(),
                TokenKind::String(s) => s.clone(),
                _ => break,
            };
            self.advance();

            // Expect colon
            let Some(token) = self.current() else {
                result.insert(key, Value::Null);
                break;
            };
            if matches!(token.kind, TokenKind::Colon) {
                self.advance();
            }

            // Parse value
            let value = self.parse_value(0)?;
            result.insert(key, value);

            // Skip comma
            if let Some(token) = self.current() {
                if matches!(token.kind, TokenKind::Comma) {
                    self.advance();
                }
            }
        }

        Ok(Value::Object(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = "level: 5";
        let mut parser = NeonParser::new(input);
        let result = parser.parse().unwrap();

        if let Value::Object(map) = result {
            assert_eq!(map.get("level"), Some(&Value::Integer(5)));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_multiple_keys() {
        let input = "a: 1\nb: 2\nc: 3";
        let mut parser = NeonParser::new(input);
        let result = parser.parse().unwrap();

        if let Value::Object(map) = result {
            assert_eq!(map.get("a"), Some(&Value::Integer(1)));
            assert_eq!(map.get("b"), Some(&Value::Integer(2)));
            assert_eq!(map.get("c"), Some(&Value::Integer(3)));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_nested() {
        let input = r#"
parameters:
    level: 5
    strict: true
"#;
        let mut parser = NeonParser::new(input);
        let result = parser.parse().unwrap();

        if let Value::Object(map) = result {
            if let Some(Value::Object(params)) = map.get("parameters") {
                assert_eq!(params.get("level"), Some(&Value::Integer(5)));
                assert_eq!(params.get("strict"), Some(&Value::Bool(true)));
            } else {
                panic!("Expected object for parameters");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_block_array() {
        let input = r#"
paths:
    - src/
    - tests/
"#;
        let mut parser = NeonParser::new(input);
        let result = parser.parse().unwrap();

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
    fn test_parse_includes() {
        let input = r#"
includes:
    - baseline.neon
parameters:
    level: 5
"#;
        let mut parser = NeonParser::new(input);
        let result = parser.parse().unwrap();

        if let Value::Object(map) = result {
            // Check includes array
            if let Some(Value::Array(arr)) = map.get("includes") {
                assert_eq!(arr.len(), 1);
                assert_eq!(arr[0], Value::String("baseline.neon".to_string()));
            } else {
                panic!("Expected array for includes, got {:?}", map.get("includes"));
            }

            // Check parameters object
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
    fn test_baseline_structure() {
        let input = r#"
parameters:
    ignoreErrors:
        -
            message: '#^Test message$#'
            identifier: test.identifier
            path: test.php
"#;
        let mut parser = NeonParser::new(input);
        let result = parser.parse().unwrap();

        if let Value::Object(map) = result {
            // Check parameters
            if let Some(Value::Object(params)) = map.get("parameters") {
                // Check ignoreErrors
                if let Some(Value::Array(ignore_errors)) = params.get("ignoreErrors") {
                    assert_eq!(ignore_errors.len(), 1);

                    // Check first error entry
                    if let Value::Object(first_error) = &ignore_errors[0] {
                        assert_eq!(first_error.get("message"), Some(&Value::String("#^Test message$#".to_string())));
                        assert_eq!(first_error.get("identifier"), Some(&Value::String("test.identifier".to_string())));
                        assert_eq!(first_error.get("path"), Some(&Value::String("test.php".to_string())));
                    } else {
                        panic!("Expected object for first ignore error");
                    }
                } else {
                    panic!("Expected array for ignoreErrors");
                }
            } else {
                panic!("Expected object for parameters");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_baseline_with_tabs() {
        // Use actual tabs like PHPStan baseline files do
        let input = "parameters:\n\tignoreErrors:\n\t\t-\n\t\t\tmessage: '#^Test$#'\n\t\t\tidentifier: test.id\n\t\t\tpath: test.php\n";
        let mut parser = NeonParser::new(input);
        let result = parser.parse().unwrap();

        if let Value::Object(map) = result {
            eprintln!("DEBUG: Parsed keys: {:?}", map.keys().collect::<Vec<_>>());
            // Check parameters
            if let Some(Value::Object(params)) = map.get("parameters") {
                eprintln!("DEBUG: Parameters keys: {:?}", params.keys().collect::<Vec<_>>());
                // Check ignoreErrors
                if let Some(Value::Array(ignore_errors)) = params.get("ignoreErrors") {
                    assert_eq!(ignore_errors.len(), 1);
                    eprintln!("DEBUG: Successfully parsed 1 ignore error");
                } else {
                    panic!("Expected array for ignoreErrors, got {:?}", params.get("ignoreErrors"));
                }
            } else {
                panic!("Expected object for parameters, got {:?}", map.get("parameters"));
            }
        } else {
            panic!("Expected object, got {:?}", result);
        }
    }

    #[test]
    fn test_real_baseline_first_100_lines() {
        use std::fs;

        let baseline_path = "/Users/borisyv/code/payjoy_www/phpstan-baseline.neon";
        let content = fs::read_to_string(baseline_path).unwrap();

        // Test with different line counts to find where it breaks
        for line_count in [5000, 10000, 20000, 50000, 100000, 121828] {
            let lines: Vec<&str> = content.lines().take(line_count).collect();
            let partial_content = lines.join("\n");

            eprintln!("\n=== Testing with {} lines ({} bytes) ===", lines.len(), partial_content.len());

            let mut parser = NeonParser::new(&partial_content);
            let result = parser.parse().unwrap();

            match result {
                Value::Object(ref map) => {
                    eprintln!("Result is Object with {} keys", map.len());
                    if map.len() == 1 && map.contains_key("parameters") {
                        eprintln!("✓ Correct structure");
                    } else {
                        eprintln!("✗ Wrong structure! Keys: {:?}", map.keys().take(10).collect::<Vec<_>>());
                        if line_count >= 500 {
                            break; // Stop testing at first failure
                        }
                    }
                }
                Value::Array(ref arr) => {
                    eprintln!("✗ Result is Array with {} entries", arr.len());
                    break;
                }
                _ => {
                    eprintln!("✗ Unexpected type");
                    break;
                }
            }
        }
    }

    #[test]
    fn test_real_baseline_full() {
        use std::fs;

        let baseline_path = "/Users/borisyv/code/payjoy_www/phpstan-baseline.neon";
        let content = fs::read_to_string(baseline_path).unwrap();

        eprintln!("DEBUG: Parsing full baseline ({} bytes)", content.len());

        let mut parser = NeonParser::new(&content);
        let result = parser.parse().unwrap();

        match result {
            Value::Object(ref map) => {
                eprintln!("DEBUG: Result is Object with {} keys", map.len());
                eprintln!("DEBUG: Keys (first 5): {:?}", map.keys().take(5).collect::<Vec<_>>());

                // Check if we have the expected structure
                assert_eq!(map.len(), 1, "Should have exactly 1 top-level key");
                assert!(map.contains_key("parameters"), "Should have 'parameters' key");

                // Check parameters structure
                if let Some(Value::Object(params)) = map.get("parameters") {
                    eprintln!("DEBUG: parameters has {} keys", params.len());
                    assert!(params.contains_key("ignoreErrors"), "Should have 'ignoreErrors' key");

                    // Check ignoreErrors is an array
                    if let Some(Value::Array(errors)) = params.get("ignoreErrors") {
                        eprintln!("DEBUG: ignoreErrors has {} entries", errors.len());
                        assert!(errors.len() > 1000, "Should have many error entries");
                    } else {
                        panic!("ignoreErrors should be an array");
                    }
                } else {
                    panic!("parameters should be an object");
                }
            }
            Value::Array(ref arr) => {
                panic!("Result is Array with {} entries (expected Object!)", arr.len());
            }
            _ => {
                panic!("Result is unexpected type: {:?}", result);
            }
        }
    }
}
