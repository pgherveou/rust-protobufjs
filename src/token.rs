use crate::parse_error::{ParseError, TokenError};
use derive_more::Display;

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

    #[display(fmt = "[")]
    OpenBracket,

    #[display(fmt = "]")]
    CloseBracket,

    #[display(fmt = "<")]
    OpenAngularBracket,

    #[display(fmt = ">")]
    CloseAngularBracket,

    #[display(fmt = ",")]
    Comma,

    Returns,
    Syntax,
    Import,
    Option,
    Service,
    Rpc,
    Stream,
    Repeated,
    Map,
    Package,
    Message,
    Enum,
    Reserved,
    Oneof,

    #[display(fmt = "\"{}\"", _0)]
    QuotedString(String),

    #[display(fmt = "{}", _0)]
    Word(String),

    #[display(fmt = "{}", _0)]
    Error(TokenError),
}

impl Token {
    pub fn as_word(self) -> Result<String, ParseError> {
        match self {
            Token::Word(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    pub fn as_quoted_string(self) -> Result<String, ParseError> {
        match self {
            Token::QuotedString(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }
}

impl From<char> for Token {
    fn from(_: char) -> Self {
        todo!()
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
