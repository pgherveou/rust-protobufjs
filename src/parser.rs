use std::ptr::NonNull;

use crate::{
    message::{Field, Message},
    namespace::Namespace,
    parse_error::{ParseError, ParseFileError},
    service::{MethodDefinition, Rpc, Service},
    token::{StringDelimiter, StringWithDelimiter, Token},
    tokenizer::Tokenizer,
};

pub struct Parser<'a> {
    file_name: &'a str,
    content: &'a str,
    tokenizer: Tokenizer,
    root: Box<Namespace>,
    package: Option<NonNull<Namespace>>,
}

impl<'a> Parser<'a> {
    pub fn new(file_name: &'a str, content: &'a str) -> Self {
        Self {
            file_name,
            content,
            tokenizer: Tokenizer::new(content),
            root: Namespace::root(),
            package: None,
        }
    }

    pub fn parse(&mut self) -> Result<(), ParseFileError> {
        self.parse_internal().map_err(|error| {
            ParseFileError::new(
                self.file_name,
                self.content,
                self.tokenizer.current_position(),
                error,
            )
        })?;
        Ok(())
    }

    fn parse_internal(&mut self) -> Result<(), ParseError> {
        while let Some(token) = self.tokenizer.next() {
            match token {
                Token::Package => self.parse_package()?,
                Token::Import => self.parse_import()?,
                Token::Syntax => self.parse_syntax()?,
                Token::Option => self.parse_option()?,
                Token::Service => {
                    let service = self.parse_service()?;
                    self.package_mut()?.add_service(service);
                }
                Token::Message => {
                    let message = self.parse_message()?;
                    self.package_mut()?.add_message(message);
                }
                Token::Error(err) => return Err(ParseError::TokenError(err)),
                token => return Err(ParseError::UnexpectedTopLevelToken(token)),
            }
        }

        return Ok(());
    }

    fn package_mut(&mut self) -> Result<&mut Namespace, ParseError> {
        self.package
            .as_mut()
            .map(|x| unsafe { x.as_mut() })
            .ok_or(ParseError::PackageNotSet)
    }

    fn parse_package(&mut self) -> Result<(), ParseError> {
        if self.package.is_some() {
            return Err(ParseError::PackageAlreadySet);
        }

        let name = self.read_unquoted_string()?;
        self.package = self.root.define(name.as_str()).as_ptr();
        self.expect_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_import(&mut self) -> Result<(), ParseError> {
        self.read_quoted_string()?;
        self.expect_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_syntax(&mut self) -> Result<(), ParseError> {
        self.expect_token(Token::Equal)?;
        self.read_quoted_string()?;
        self.expect_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_option(&mut self) -> Result<(), ParseError> {
        self.read_unquoted_string()?;
        self.expect_token(Token::Equal)?;
        self.read_string()?;
        self.expect_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_message(&mut self) -> Result<Message, ParseError> {
        let message_name = self.read_unquoted_string()?;
        self.expect_token(Token::OpenCurlyBracket)?;

        let mut message = Message::new(message_name);

        loop {
            match self.tokenizer.read_token()? {
                Token::CloseCurlyBracket => {
                    break;
                }
                Token::Message => {
                    message.add_nested(self.parse_message()?);
                    self.expect_token(Token::SemiColon)?;
                }
                Token::Reserved => {
                    self.parse_reserved()?;
                }
                Token::Option => {
                    self.parse_message_option()?;
                }
                Token::Repeated => {
                    let type_name = self.read_unquoted_string()?;
                    self.parse_message_field(&mut message, type_name, true)?;
                }
                Token::String(type_name) => {
                    type_name.expect_delimiter(StringDelimiter::None)?;
                    self.parse_message_field(&mut message, type_name.into(), false)?;
                }
                token => return Err(ParseError::UnexpectedMessageToken(token)),
            }
        }

        Ok(message)
    }

    fn parse_service(&mut self) -> Result<Service, ParseError> {
        let name = self.read_unquoted_string()?;
        let mut service = Service::new(name);

        self.expect_token(Token::OpenCurlyBracket)?;

        loop {
            match self.tokenizer.read_token()? {
                Token::CloseCurlyBracket => {
                    break;
                }
                Token::Rpc => {
                    let rpc = self.parse_rpc()?;
                    service.add_rpc(rpc)
                }

                token => return Err(ParseError::UnexpectedMessageToken(token)),
            }
        }

        Ok(service)
    }

    fn parse_rpc(&mut self) -> Result<Rpc, ParseError> {
        let name = self.read_unquoted_string()?;

        self.expect_token(Token::OpenParenthesis)?;

        let request = match self.tokenizer.read_token()? {
            Token::Stream => MethodDefinition::new(self.read_unquoted_string()?, true),
            token => MethodDefinition::new(token.as_unquoted_string()?, false),
        };

        self.expect_token(Token::CloseParenthesis)?;
        self.expect_token(Token::Returns)?;
        self.expect_token(Token::OpenParenthesis)?;

        let response = match self.tokenizer.read_token()? {
            Token::Stream => MethodDefinition::new(self.read_unquoted_string()?, true),
            token => MethodDefinition::new(token.as_unquoted_string()?, false),
        };

        self.expect_token(Token::CloseParenthesis)?;
        self.expect_token(Token::OpenCurlyBracket)?;
        self.tokenizer.skip_until_token(Token::CloseCurlyBracket)?;

        Ok(Rpc::new(name, request, response))
    }

    fn parse_message_field(
        &mut self,
        message: &mut Message,
        type_name: String,
        repeated: bool,
    ) -> Result<(), ParseError> {
        let field_name = self.read_unquoted_string()?;
        self.expect_token(Token::Equal)?;
        let field_id = self
            .read_unquoted_string()?
            .parse::<u32>()
            .map_err(|err| ParseError::ParseFieldId(err))?;
        message.add_field(field_name, Field::new(field_id, type_name, repeated));
        self.expect_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_message_option(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_reserved(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::SemiColon)?;
        Ok(())
    }

    fn read_string(&mut self) -> Result<StringWithDelimiter, ParseError> {
        match self.tokenizer.read_token()? {
            Token::String(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    fn read_quoted_string(&mut self) -> Result<String, ParseError> {
        let str = self.read_string()?;
        str.expect_delimiter(StringDelimiter::DoubleQuote)?;
        Ok(str.into())
    }

    fn read_unquoted_string(&mut self) -> Result<String, ParseError> {
        let str = self.read_string()?;
        str.expect_delimiter(StringDelimiter::None)?;
        Ok(str.into())
    }

    fn expect_token(&mut self, expected: Token) -> Result<(), ParseError> {
        let received = self.tokenizer.read_token()?;
        if received == expected {
            return Ok(());
        }
        Err(ParseError::UnexpectedToken { expected, received })
    }

    pub fn print(&self) {
        println!("{}", self.root.fullname)
    }
}

#[cfg(test)]
mod tests {
    use super::Parser;

    #[test]
    fn it_should_parse_sample_file() {
        let src = r#"
syntax = "proto3";

package pb.hello;

option go_package = "hello";
option java_package = "com.hello.service.api.v1";
option java_multiple_files = true;
option py_generic_services = true;

service HelloWorld {
    rpc SayHello (SayHelloRequest) returns (SayHelloResponse) {}
    rpc LotsOfReplies(SayHelloRequest) returns (stream SayHelloResponse) {}
    rpc LotsOfGreetings(stream SayHelloRequest) returns (SayHelloResponses) {}
    rpc BidiHello(stream SayHelloRequest) returns (stream SayHelloResponse) {}
}

message SayHelloRequest {
    string name = 1;
    string phone = 2;
}

message SayHelloResponse {
    string hello = 1;
}

message SayHelloResponses {
    repeated SayHelloResponse responses = 1;
}
"#;

        let mut parser = Parser::new("test_file.proto", src);

        match parser.parse() {
            Ok(_) => println!("result {:?}", parser.root),
            Err(err) => println!("error:\n{}", err),
        }
    }
}
