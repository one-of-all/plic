use crate::ast::Literal;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnexpectedChar(char, usize),
    UnterminatedString(usize),
    InvalidEscape(char, usize),
    InvalidHexByteString(String, usize),
    InvalidNumber(String, usize),
    InvalidDuration(String, usize),
    UnexpectedEof,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LexError::UnexpectedChar(c, pos) => write!(f, "unexpected character '{}' at position {}", c, pos),
            LexError::UnterminatedString(pos) => write!(f, "unterminated string literal at position {}", pos),
            LexError::InvalidEscape(c, pos) => write!(f, "invalid escape sequence '\\{}' at position {}", c, pos),
            LexError::InvalidHexByteString(s, pos) => write!(f, "invalid hex byte string '{}' at position {}", s, pos),
            LexError::InvalidNumber(s, pos) => write!(f, "invalid number '{}' at position {}", s, pos),
            LexError::InvalidDuration(s, pos) => write!(f, "invalid duration '{}' at position {}", s, pos),
            LexError::UnexpectedEof => write!(f, "unexpected end of input"),
        }
    }
}

impl std::error::Error for LexError {}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    INDENT,
    DEDENT,
    NEWLINE,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Pipe,
    Arrow,
    FatArrow,
    Assign,
    Colon,
    DoubleColon,
    Cons,
    Concat,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Not,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Dollar,
    Dot,
    DotDot,
    If,
    Then,
    Else,
    Let,
    In,
    Case,
    Of,
    Data,
    Struct,
    Try,
    Catch,
    Error,
    For,
    While,
    Lambda,
    Class,
    Extends,
    New,
    Loop,
    Break,
    Super,      // new token for `super`
    Ident(String),
    Literal(Literal),
    FString(String),
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn span(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    indent_stack: Vec<usize>,
    at_line_start: bool,
    line_start_spaces: usize,
    pending_tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            chars: input.chars().collect(),
            pos: 0,
            indent_stack: vec![0],
            at_line_start: true,
            line_start_spaces: 0,
            pending_tokens: Vec::new(),
        }
    }

    fn current(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            match self.next_token()? {
                Some(Token { kind: TokenKind::Eof, start, end }) => {
                    while self.indent_stack.len() > 1 {
                        self.indent_stack.pop();
                        tokens.push(Token { kind: TokenKind::DEDENT, start, end });
                    }
                    tokens.push(Token { kind: TokenKind::Eof, start, end });
                    break;
                }
                Some(tok) => tokens.push(tok),
                None => break,
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Option<Token>, LexError> {
        if let Some(tok) = self.pending_tokens.pop() {
            return Ok(Some(tok));
        }

        self.skip_whitespace()?;

        let c = match self.current() {
            Some(ch) => ch,
            None => return Ok(None),
        };

        let start = self.pos;

        if c == '\n' {
            self.advance();
            let end = self.pos;
            self.at_line_start = true;
            return Ok(Some(Token { kind: TokenKind::NEWLINE, start, end }));
        }

        if self.at_line_start {
            self.line_start_spaces = 0;
            while let Some(' ') = self.current() {
                self.line_start_spaces += 1;
                self.advance();
            }
            self.at_line_start = false;
            let current_indent = *self.indent_stack.last().unwrap();
            let end = self.pos;
            if self.line_start_spaces > current_indent {
                self.indent_stack.push(self.line_start_spaces);
                return Ok(Some(Token { kind: TokenKind::INDENT, start, end }));
            } else if self.line_start_spaces < current_indent {
                let mut dedents = Vec::new();
                while self.indent_stack.last().map(|&x| x > self.line_start_spaces).unwrap_or(false) {
                    self.indent_stack.pop();
                    dedents.push(Token { kind: TokenKind::DEDENT, start, end });
                }
                self.pending_tokens.extend(dedents.into_iter().rev());
                if let Some(tok) = self.pending_tokens.pop() {
                    return Ok(Some(tok));
                }
            }
        }

        self.read_token(start)
    }

    fn read_token(&mut self, start: usize) -> Result<Option<Token>, LexError> {
        let c = self.current().unwrap();
        let end = self.pos + 1;
        match c {
            '(' => { self.advance(); Ok(Some(Token { kind: TokenKind::LParen, start, end })) }
            ')' => { self.advance(); Ok(Some(Token { kind: TokenKind::RParen, start, end })) }
            '{' => { self.advance(); Ok(Some(Token { kind: TokenKind::LBrace, start, end })) }
            '}' => { self.advance(); Ok(Some(Token { kind: TokenKind::RBrace, start, end })) }
            '[' => { self.advance(); Ok(Some(Token { kind: TokenKind::LBracket, start, end })) }
            ']' => { self.advance(); Ok(Some(Token { kind: TokenKind::RBracket, start, end })) }
            ',' => { self.advance(); Ok(Some(Token { kind: TokenKind::Comma, start, end })) }
            ';' => { self.advance(); Ok(Some(Token { kind: TokenKind::Semicolon, start, end })) }
            '|' => {
                self.advance();
                if self.current() == Some('>') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::Pipe, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Pipe, start, end }))
                }
            }
            '=' => {
                self.advance();
                if self.current() == Some('>') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::FatArrow, start, end }))
                } else if self.current() == Some('=') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::Eq, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Assign, start, end }))
                }
            }
            ':' => {
                self.advance();
                if self.current() == Some(':') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::DoubleColon, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Colon, start, end }))
                }
            }
            '+' => {
                self.advance();
                if self.current() == Some('+') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::Concat, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Plus, start, end }))
                }
            }
            '-' => {
                self.advance();
                if self.current() == Some('>') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::Arrow, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Minus, start, end }))
                }
            }
            '*' => { self.advance(); Ok(Some(Token { kind: TokenKind::Star, start, end })) }
            '/' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::Neq, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Slash, start, end }))
                }
            }
            '%' => { self.advance(); Ok(Some(Token { kind: TokenKind::Percent, start, end })) }
            '!' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::Neq, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Not, start, end }))
                }
            }
            '<' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::Le, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Lt, start, end }))
                }
            }
            '>' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::Ge, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Gt, start, end }))
                }
            }
            '&' => { self.advance(); Ok(Some(Token { kind: TokenKind::And, start, end })) }
            '$' => { self.advance(); Ok(Some(Token { kind: TokenKind::Dollar, start, end })) }
            '.' => {
                self.advance();
                if self.current() == Some('.') {
                    self.advance();
                    Ok(Some(Token { kind: TokenKind::DotDot, start, end }))
                } else {
                    Ok(Some(Token { kind: TokenKind::Dot, start, end }))
                }
            }
            '"' => self.consume_string(start),
            '\'' => self.consume_char(start),
            '@' => self.consume_uid(start),
            '#' => self.consume_byte_string(start),
            c if c.is_ascii_digit() => self.consume_number_or_duration(start, c),
            c if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();
                ident.push(c);
                self.advance();
                while let Some(ch) = self.current() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let end = self.pos;
                if ident == "f" && self.current() == Some('"') {
                    self.advance();
                    return self.consume_fstring(start);
                }
                match ident.as_str() {
                    "true" => Ok(Some(Token { kind: TokenKind::Literal(Literal::Bool(true)), start, end })),
                    "false" => Ok(Some(Token { kind: TokenKind::Literal(Literal::Bool(false)), start, end })),
                    "if" => Ok(Some(Token { kind: TokenKind::If, start, end })),
                    "then" => Ok(Some(Token { kind: TokenKind::Then, start, end })),
                    "else" => Ok(Some(Token { kind: TokenKind::Else, start, end })),
                    "let" => Ok(Some(Token { kind: TokenKind::Let, start, end })),
                    "in" => Ok(Some(Token { kind: TokenKind::In, start, end })),
                    "case" => Ok(Some(Token { kind: TokenKind::Case, start, end })),
                    "of" => Ok(Some(Token { kind: TokenKind::Of, start, end })),
                    "data" => Ok(Some(Token { kind: TokenKind::Data, start, end })),
                    "struct" => Ok(Some(Token { kind: TokenKind::Struct, start, end })),
                    "try" => Ok(Some(Token { kind: TokenKind::Try, start, end })),
                    "catch" => Ok(Some(Token { kind: TokenKind::Catch, start, end })),
                    "error" => Ok(Some(Token { kind: TokenKind::Error, start, end })),
                    "for" => Ok(Some(Token { kind: TokenKind::For, start, end })),
                    "while" => Ok(Some(Token { kind: TokenKind::While, start, end })),
                    "lambda" => Ok(Some(Token { kind: TokenKind::Lambda, start, end })),
                    "and" => Ok(Some(Token { kind: TokenKind::And, start, end })),
                    "or" => Ok(Some(Token { kind: TokenKind::Or, start, end })),
                    "not" => Ok(Some(Token { kind: TokenKind::Not, start, end })),
                    "class" => Ok(Some(Token { kind: TokenKind::Class, start, end })),
                    "extends" => Ok(Some(Token { kind: TokenKind::Extends, start, end })),
                    "new" => Ok(Some(Token { kind: TokenKind::New, start, end })),
                    "loop" => Ok(Some(Token { kind: TokenKind::Loop, start, end })),
                    "break" => Ok(Some(Token { kind: TokenKind::Break, start, end })),
                    "super" => Ok(Some(Token { kind: TokenKind::Super, start, end })),
                    _ => Ok(Some(Token { kind: TokenKind::Ident(ident), start, end })),
                }
            }
            _ => Err(LexError::UnexpectedChar(c, self.pos)),
        }
    }

    fn consume_string(&mut self, start: usize) -> Result<Option<Token>, LexError> {
        self.advance();
        let mut s = String::new();
        while let Some(ch) = self.current() {
            if ch == '"' {
                self.advance();
                let end = self.pos;
                return Ok(Some(Token { kind: TokenKind::Literal(Literal::String(s)), start, end }));
            }
            if ch == '\\' {
                self.advance();
                if let Some(esc) = self.current() {
                    match esc {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        '\'' => s.push('\''),
                        _ => return Err(LexError::InvalidEscape(esc, self.pos)),
                    }
                    self.advance();
                } else {
                    return Err(LexError::UnexpectedEof);
                }
            } else {
                s.push(ch);
                self.advance();
            }
        }
        Err(LexError::UnterminatedString(start))
    }

    fn consume_fstring(&mut self, start: usize) -> Result<Option<Token>, LexError> {
        let mut content = String::new();
        while let Some(ch) = self.current() {
            if ch == '"' {
                if let Some(prev) = self.chars.get(self.pos - 1) {
                    if *prev == '\\' {
                        content.push('"');
                        self.advance();
                        continue;
                    }
                }
                self.advance();
                let end = self.pos;
                return Ok(Some(Token { kind: TokenKind::FString(content), start, end }));
            }
            if ch == '\\' && self.current().and_then(|_| self.chars.get(self.pos + 1)).map(|&c| c == '"').unwrap_or(false) {
                content.push('\\');
                self.advance();
                if let Some(ch2) = self.current() {
                    content.push(ch2);
                    self.advance();
                }
                continue;
            }
            content.push(ch);
            self.advance();
        }
        Err(LexError::UnterminatedString(start))
    }

    fn consume_char(&mut self, start: usize) -> Result<Option<Token>, LexError> {
        self.advance();
        let ch = if self.current() == Some('\\') {
            self.advance();
            if let Some(esc) = self.current() {
                self.advance();
                match esc {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '\'' => '\'',
                    _ => return Err(LexError::InvalidEscape(esc, self.pos)),
                }
            } else {
                return Err(LexError::UnexpectedEof);
            }
        } else {
            if let Some(c) = self.current() {
                self.advance();
                c
            } else {
                return Err(LexError::UnexpectedEof);
            }
        };
        if self.current() == Some('\'') {
            self.advance();
            let end = self.pos;
            Ok(Some(Token { kind: TokenKind::Literal(Literal::Char(ch)), start, end }))
        } else {
            Err(LexError::UnterminatedString(start))
        }
    }

    fn consume_uid(&mut self, start: usize) -> Result<Option<Token>, LexError> {
        self.advance();
        let mut uid = String::from("@");
        while let Some(ch) = self.current() {
            if ch.is_alphanumeric() || ch == '_' {
                uid.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        let end = self.pos;
        Ok(Some(Token { kind: TokenKind::Literal(Literal::Uid(uid)), start, end }))
    }

    fn consume_byte_string(&mut self, start: usize) -> Result<Option<Token>, LexError> {
        self.advance();
        if self.current() == Some('B') {
            self.advance();
            if self.current() != Some('"') {
                return Err(LexError::UnexpectedChar(self.current().unwrap_or('\0'), self.pos));
            }
            self.advance();
            let mut hex = String::new();
            while let Some(ch) = self.current() {
                if ch == '"' {
                    self.advance();
                    let end = self.pos;
                    let bytes = hex::decode(&hex)
                        .map_err(|_| LexError::InvalidHexByteString(hex.clone(), self.pos))?;
                    return Ok(Some(Token { kind: TokenKind::Literal(Literal::ByteString(bytes)), start, end }));
                }
                hex.push(ch);
                self.advance();
            }
            Err(LexError::UnterminatedString(self.pos))
        } else {
            Err(LexError::UnexpectedChar('#', self.pos))
        }
    }

    fn consume_number_or_duration(&mut self, start: usize, first: char) -> Result<Option<Token>, LexError> {
        let mut num_str = String::new();
        num_str.push(first);
        self.advance();
        let mut is_float = false;
        loop {
            match self.current() {
                Some(ch) if ch.is_ascii_digit() => {
                    num_str.push(ch);
                    self.advance();
                }
                Some('.') => {
                    if let Some(&next) = self.chars.get(self.pos + 1) {
                        if next == '.' {
                            break;
                        }
                    }
                    is_float = true;
                    num_str.push('.');
                    self.advance();
                }
                _ => break,
            }
        }

        let suffix = match self.current() {
            Some('s') => {
                self.advance();
                Some('s')
            }
            Some('m') => {
                self.advance();
                if self.current() == Some('s') {
                    self.advance();
                    Some('m')
                } else {
                    Some('M')
                }
            }
            Some('h') => {
                self.advance();
                Some('h')
            }
            _ => None,
        };
        let end = self.pos;

        if let Some(suf) = suffix {
            let num: f64 = num_str.parse().map_err(|_| LexError::InvalidNumber(num_str.clone(), start))?;
            let (secs, nanos) = match suf {
                's' => {
                    let secs = num as u64;
                    let nanos = ((num - secs as f64) * 1_000_000_000.0) as u32;
                    (secs, nanos)
                }
                'm' => {
                    let millis = (num * 1000.0) as u64;
                    let secs = millis / 1000;
                    let nanos = ((millis % 1000) * 1_000_000) as u32;
                    (secs, nanos)
                }
                'M' => {
                    let secs = (num * 60.0) as u64;
                    let nanos = ((num * 60.0 - secs as f64) * 1_000_000_000.0) as u32;
                    (secs, nanos)
                }
                'h' => {
                    let secs = (num * 3600.0) as u64;
                    let nanos = ((num * 3600.0 - secs as f64) * 1_000_000_000.0) as u32;
                    (secs, nanos)
                }
                _ => return Err(LexError::InvalidDuration(num_str, start)),
            };
            let dur = Duration::new(secs, nanos);
            Ok(Some(Token { kind: TokenKind::Literal(Literal::Duration(dur)), start, end }))
        } else {
            if is_float {
                let f: f64 = num_str.parse().map_err(|_| LexError::InvalidNumber(num_str.clone(), start))?;
                Ok(Some(Token { kind: TokenKind::Literal(Literal::Float(f)), start, end }))
            } else {
                let i: i64 = num_str.parse().map_err(|_| LexError::InvalidNumber(num_str.clone(), start))?;
                Ok(Some(Token { kind: TokenKind::Literal(Literal::Int(i)), start, end }))
            }
        }
    }

    fn skip_whitespace(&mut self) -> Result<(), LexError> {
        while let Some(ch) = self.current() {
            match ch {
                ' ' | '\t' | '\r' => self.advance(),
                _ => break,
            }
        }
        Ok(())
    }
}
