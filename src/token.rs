use std::convert::TryFrom;

use crate::parse_error::{ParseError, TokenError};
use derive_more::Display;

#[derive(Display, Clone, Debug, PartialEq)]
pub enum StringDelimiter {
    #[display(fmt = "\"")]
    DoubleQuote,

    #[display(fmt = "'")]
    SingleQuote,

    #[display(fmt = "")]
    None,
}

#[derive(Display, Debug, PartialEq)]
#[display(fmt = "{}", _0)]
pub struct StringWithDelimiter(String, StringDelimiter);

impl StringWithDelimiter {
    pub fn new(str: String, delimiter: StringDelimiter) -> Self {
        Self(str, delimiter)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn expect_delimiter(&self, v: StringDelimiter) -> Result<(), ParseError> {
        if self.1 == v {
            return Ok(());
        }

        return Err(ParseError::UnexpectedStringDelimiter(self.1.clone()));
    }
}

impl Into<String> for StringWithDelimiter {
    fn into(self) -> String {
        self.0
    }
}

impl From<&str> for StringWithDelimiter {
    fn from(s: &str) -> Self {
        StringWithDelimiter(s.to_string(), StringDelimiter::None)
    }
}

impl TryFrom<char> for StringDelimiter {
    type Error = TokenError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            '\'' => Ok(StringDelimiter::SingleQuote),
            '"' => Ok(StringDelimiter::DoubleQuote),
            c => Err(TokenError::InvalidStringDelimiter(c)),
        }
    }
}

#[derive(Display, Debug, PartialEq)]
pub enum Token {
    #[display(fmt = "=")]
    Equal,

    #[display(fmt = ";")]
    SemiColon,

    #[display(fmt = "{{")]
    OpenCurlyBracket,

    #[display(fmt = "}}")]
    CloseCurlyBracket,

    #[display(fmt = "(")]
    OpenParenthesis,

    #[display(fmt = ")")]
    CloseParenthesis,

    Returns,

    Syntax,
    Import,
    Option,
    Service,
    Rpc,
    Stream,
    Repeated,
    Package,
    Message,
    Reserved,

    #[display(fmt = "{}", _0)]
    String(StringWithDelimiter),

    #[display(fmt = "{}", _0)]
    Error(TokenError),
}

impl Token {
    pub fn as_unquoted_string(self) -> Result<String, ParseError> {
        match self {
            Token::String(v) => {
                v.expect_delimiter(StringDelimiter::None)?;
                Ok(v.into())
            }
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    pub fn as_quoted_string(self) -> Result<String, ParseError> {
        match self {
            Token::String(v) => {
                v.expect_delimiter(StringDelimiter::DoubleQuote)?;
                Ok(v.into())
            }
            token => Err(ParseError::UnexpectedString(token)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::token::Token;

    #[test]
    fn it_should_display() {
        let s = Token::SemiColon.to_string();
        println!("{}", s)
    }
}
