use crate::parse_error::ParseError;
use derive_more::Display;

#[derive(Display, Debug, PartialEq)]
pub enum Token {
    #[display(fmt = "EOF")]
    EOF,

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
    Public,
    Option,
    Service,
    Rpc,
    Stream,
    Repeated,
    Map,
    Package,
    Optional,
    Message,
    Extend,
    Enum,
    Reserved,
    Oneof,

    #[display(fmt = "\"{}\"", _0)]
    QuotedString(String),

    #[display(fmt = "{}", _0)]
    Word(String),
}

impl Token {
    pub fn as_word(self) -> Result<String, ParseError> {
        match self {
            Token::Word(v) => Ok(v),
            // Token::Import => Ok("import".to_string()),
            Token::Package => Ok("package".to_string()),
            Token::Reserved => Ok("reserved".to_string()),
            Token::Option => Ok("option".to_string()),
            Token::Service => Ok("service".to_string()),
            Token::Public => Ok("public".to_string()),
            Token::Enum => Ok("enum".to_string()),
            // Token::Rpc => Ok("rpc".to_string()),
            // Token::Stream => Ok("stream".to_string()),
            // Token::Repeated => Ok("repeated".to_string()),
            Token::Map => Ok("map".to_string()),
            Token::Message => Ok("message".to_string()),
            Token::Syntax => Ok("syntax".to_string()),
            // Token::Oneof => Ok("oneof".to_string()),
            // Token::Enum => Ok("enum".to_string()),
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
