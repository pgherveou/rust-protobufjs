use crate::{field::FieldRule, parse_error::ParseError};
use derive_more::Display;

// TODO add lifetime and take &'a str instead of String
#[derive(Display, Debug, PartialEq)]
pub enum Token {
    #[display(fmt = "EOF")]
    EOF,

    #[display(fmt = "=")]
    Eq,

    #[display(fmt = ";")]
    Semi,

    #[display(fmt = ":")]
    Colon,

    #[display(fmt = "{{")]
    LBrace,

    #[display(fmt = "}}")]
    RBrace,

    #[display(fmt = "(")]
    LParen,

    #[display(fmt = ")")]
    RParen,

    #[display(fmt = "[")]
    LBrack,

    #[display(fmt = "]")]
    RBrack,

    #[display(fmt = "<")]
    LAngle,

    #[display(fmt = ">")]
    Rangle,

    #[display(fmt = ",")]
    Comma,

    Returns,
    Syntax,
    Import,
    Public,
    Option,
    Service,
    Rpc,
    Stream,
    FieldRule(FieldRule),
    Extensions,
    Map,
    Package,
    Message,
    Extend,
    Enum,
    Reserved,
    Oneof,

    #[display(fmt = "\"{}\"", _0)]
    String(String),

    #[display(fmt = "{}", _0)]
    Identifier(String),
}

impl Token {
    pub fn identifier(self) -> Result<String, ParseError> {
        match self {
            Token::Identifier(v) => Ok(v),
            Token::Package => Ok("package".to_string()),
            Token::Reserved => Ok("reserved".to_string()),
            Token::Option => Ok("option".to_string()),
            Token::Service => Ok("service".to_string()),
            Token::Public => Ok("public".to_string()),
            Token::Extensions => Ok("extensions".to_string()),
            Token::Enum => Ok("enum".to_string()),
            Token::FieldRule(rule) => Ok(rule.to_string()),
            Token::Map => Ok("map".to_string()),
            Token::Message => Ok("message".to_string()),
            Token::Syntax => Ok("syntax".to_string()),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    pub fn into_quoted_string(self) -> Result<String, ParseError> {
        match self {
            Token::String(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }
}

impl From<char> for Token {
    fn from(_: char) -> Self {
        todo!()
    }
}
