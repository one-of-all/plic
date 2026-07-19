use crate::ast::Span;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ChatError {
    pub message: String,
    pub code: i32,
    pub span: Option<Span>,
    pub file: Option<String>,
}

impl ChatError {
    pub fn new(msg: &str, code: i32) -> Self {
        ChatError { message: msg.to_string(), code, span: None, file: None }
    }
    pub fn with_span(msg: &str, code: i32, span: Span) -> Self {
        ChatError { message: msg.to_string(), code, span: Some(span), file: None }
    }
    pub fn with_file(msg: &str, code: i32, file: String) -> Self {
        ChatError { message: msg.to_string(), code, span: None, file: Some(file) }
    }
}

impl fmt::Display for ChatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(span) = self.span {
            write!(f, "{} at {:?}", self.message, span)
        } else if let Some(file) = &self.file {
            write!(f, "{} in file {}", self.message, file)
        } else {
            write!(f, "{} (code {})", self.message, self.code)
        }
    }
}

impl From<std::io::Error> for ChatError {
    fn from(e: std::io::Error) -> Self {
        ChatError::new(&e.to_string(), 2)
    }
}

impl From<reqwest::Error> for ChatError {
    fn from(e: reqwest::Error) -> Self {
        ChatError::new(&e.to_string(), 4)
    }
}
