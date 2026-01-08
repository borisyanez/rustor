//! NEON lexer - tokenizes NEON input

use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// A key or unquoted string value
    Identifier(String),
    /// A quoted string "..." or '...'
    String(String),
    /// An integer number
    Integer(i64),
    /// A floating point number
    Float(f64),
    /// true or false
    Bool(bool),
    /// null
    Null,
    /// :
    Colon,
    /// =
    Equals,
    /// -
    Dash,
    /// ,
    Comma,
    /// [
    LeftBracket,
    /// ]
    RightBracket,
    /// {
    LeftBrace,
    /// }
    RightBrace,
    /// (
    LeftParen,
    /// )
    RightParen,
    /// Newline
    Newline,
    /// Indentation (spaces at start of line)
    Indent(usize),
    /// End of file
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    position: usize,
    line: usize,
    column: usize,
    at_line_start: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            position: 0,
            line: 1,
            column: 1,
            at_line_start: true,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.next();
        if let Some(c) = ch {
            self.position += c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.column = 1;
                self.at_line_start = true;
            } else {
                self.column += 1;
            }
        }
        ch
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    fn skip_whitespace_on_line(&mut self) {
        while let Some(&ch) = self.peek() {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_indent(&mut self) -> usize {
        let mut spaces = 0;
        while let Some(&ch) = self.peek() {
            match ch {
                ' ' => {
                    spaces += 1;
                    self.advance();
                }
                '\t' => {
                    spaces += 4; // Treat tab as 4 spaces
                    self.advance();
                }
                _ => break,
            }
        }
        spaces
    }

    fn skip_comment(&mut self) {
        while let Some(&ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn read_string(&mut self, quote: char) -> String {
        let mut result = String::new();
        // Note: opening quote was already consumed by next_token()

        while let Some(ch) = self.advance() {
            if ch == quote {
                break;
            } else if ch == '\\' {
                if let Some(escaped) = self.advance() {
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        '\'' => result.push('\''),
                        _ => {
                            result.push('\\');
                            result.push(escaped);
                        }
                    }
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn read_identifier(&mut self) -> String {
        let mut result = String::new();

        while let Some(&ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '/' || ch == '\\' || ch == '@' || ch == '*' || ch == '$' || ch == '%' {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        result
    }

    fn read_number(&mut self, first: char) -> TokenKind {
        let mut num_str = String::new();
        num_str.push(first);

        let mut is_float = false;

        while let Some(&ch) = self.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !is_float {
                num_str.push(ch);
                is_float = true;
                self.advance();
            } else {
                break;
            }
        }

        if is_float {
            TokenKind::Float(num_str.parse().unwrap_or(0.0))
        } else {
            TokenKind::Integer(num_str.parse().unwrap_or(0))
        }
    }

    pub fn next_token(&mut self) -> Token {
        // Handle indentation at line start
        if self.at_line_start {
            self.at_line_start = false;
            let indent = self.read_indent();

            // Check if line is blank or comment
            if let Some(&ch) = self.peek() {
                if ch == '\n' {
                    self.advance();
                    return Token::new(TokenKind::Newline, self.line, self.column);
                }
                if ch == '#' {
                    self.skip_comment();
                    if self.peek().is_some() {
                        self.advance(); // consume newline
                    }
                    return Token::new(TokenKind::Newline, self.line, self.column);
                }
            }

            if indent > 0 {
                return Token::new(TokenKind::Indent(indent), self.line, self.column);
            }
        }

        // Skip inline whitespace
        self.skip_whitespace_on_line();

        let line = self.line;
        let column = self.column;

        let Some(ch) = self.advance() else {
            return Token::new(TokenKind::Eof, line, column);
        };

        let kind = match ch {
            '#' => {
                self.skip_comment();
                if self.peek().is_some() {
                    self.advance();
                }
                TokenKind::Newline
            }
            '\n' => TokenKind::Newline,
            ':' => TokenKind::Colon,
            '=' => TokenKind::Equals,
            '-' => {
                // Check if this is a negative number or list item
                if let Some(&next_ch) = self.peek() {
                    if next_ch.is_ascii_digit() {
                        self.read_number('-')
                    } else {
                        TokenKind::Dash
                    }
                } else {
                    TokenKind::Dash
                }
            }
            ',' => TokenKind::Comma,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '"' | '\'' => TokenKind::String(self.read_string(ch)),
            _ if ch.is_ascii_digit() => self.read_number(ch),
            _ => {
                // Read identifier and check for keywords
                let mut ident = String::new();
                ident.push(ch);
                ident.push_str(&self.read_identifier());

                match ident.to_lowercase().as_str() {
                    "true" | "yes" | "on" => TokenKind::Bool(true),
                    "false" | "no" | "off" => TokenKind::Bool(false),
                    "null" | "none" => TokenKind::Null,
                    _ => TokenKind::Identifier(ident),
                }
            }
        };

        Token::new(kind, line, column)
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("level: 5");
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "level"));
        assert!(matches!(tokens[1].kind, TokenKind::Colon));
        assert!(matches!(tokens[2].kind, TokenKind::Integer(5)));
    }

    #[test]
    fn test_indentation() {
        let input = "key:\n    value";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "key"));
        assert!(matches!(tokens[1].kind, TokenKind::Colon));
        assert!(matches!(tokens[2].kind, TokenKind::Newline));
        assert!(matches!(tokens[3].kind, TokenKind::Indent(4)));
        assert!(matches!(tokens[4].kind, TokenKind::Identifier(ref s) if s == "value"));
    }

    #[test]
    fn test_array_dash() {
        let input = "- item1\n- item2";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Dash));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(ref s) if s == "item1"));
    }

    #[test]
    fn test_quoted_string() {
        let input = r#"key: "hello world""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "key"));
        assert!(matches!(tokens[2].kind, TokenKind::String(ref s) if s == "hello world"));
    }

    #[test]
    fn test_inline_array() {
        let input = "[1, 2, 3]";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::LeftBracket));
        assert!(matches!(tokens[1].kind, TokenKind::Integer(1)));
        assert!(matches!(tokens[2].kind, TokenKind::Comma));
        assert!(matches!(tokens[3].kind, TokenKind::Integer(2)));
    }

    #[test]
    fn test_booleans() {
        let input = "a: true\nb: false";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[2].kind, TokenKind::Bool(true)));
        assert!(matches!(tokens[6].kind, TokenKind::Bool(false)));
    }

    #[test]
    fn test_negative_number() {
        let input = "value: -42";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[2].kind, TokenKind::Integer(-42)));
    }
}
