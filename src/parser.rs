use std::ptr::NonNull;

use crate::{
    message::{EnumTuple, Field, Message, Oneof},
    namespace::Namespace,
    parse_error::{ParseError, ParseFileError},
    service::{Rpc, Service},
    token::Token,
    tokenizer::Tokenizer,
};

pub struct Parser<'a> {
    file_name: &'a str,
    content: &'a str,
    tokenizer: Tokenizer,
    pub root: Box<Namespace>,
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
                    let (name, message) = self.parse_message()?;
                    self.package_mut()?.add_message(name, message);
                }
                Token::Enum => {
                    let (name, enum_tuples) = self.parse_enum()?;
                    self.package_mut()?.add_enum(name, enum_tuples);
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

        let name = self.read_word()?;
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
        self.read_word()?;
        self.expect_token(Token::Equal)?;
        self.read_word_or_string()?;
        self.expect_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_message(&mut self) -> Result<(String, Message), ParseError> {
        let message_name = self.read_word()?;
        self.expect_token(Token::OpenCurlyBracket)?;

        let mut message = Message::new();
        let mut oneof = None;

        loop {
            match self.tokenizer.read_token()? {
                Token::CloseCurlyBracket => match oneof.take() {
                    Some(oneof) => message.add_oneof(oneof),
                    None => break,
                },
                Token::Message => {
                    let (name, nested_message) = self.parse_message()?;
                    message.add_nested(name, nested_message);
                    self.expect_token(Token::SemiColon)?;
                }
                Token::Oneof => {
                    let name = self.read_word()?;
                    oneof = Some(Oneof::new(name));
                    self.expect_token(Token::OpenCurlyBracket)?;
                }
                Token::Enum => {
                    let (name, enum_tuples) = self.parse_enum()?;
                    message.add_enum(name, enum_tuples);
                }
                Token::Reserved => {
                    self.parse_reserved()?;
                }
                Token::Option => {
                    self.parse_message_option()?;
                }
                Token::Repeated => {
                    let type_name = self.read_word()?;
                    let (name, field) = self.parse_message_field(type_name, true, None)?;
                    message.add_field(name, field);
                }
                Token::Map => {
                    self.expect_token(Token::OpenAngularBracket)?;
                    let key_type = self.read_word()?;
                    self.expect_token(Token::Comma)?;
                    let type_name = self.read_word()?;
                    self.expect_token(Token::CloseAngularBracket)?;
                    let (name, field) =
                        self.parse_message_field(type_name, true, Some(key_type))?;
                    message.add_field(name, field);
                }
                Token::Word(type_name) => {
                    let (name, field) = self.parse_message_field(type_name, false, None)?;
                    match oneof {
                        Some(ref mut oneof) => oneof.add_field_name(name.to_string()),
                        None => {}
                    }

                    message.add_field(name, field);
                }
                token => return Err(ParseError::UnexpectedMessageToken(token)),
            }
        }

        Ok((message_name, message))
    }

    fn parse_service(&mut self) -> Result<Service, ParseError> {
        let name = self.read_word()?;
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
        let name = self.read_word()?;

        self.expect_token(Token::OpenParenthesis)?;

        let (request_type, request_stream) = match self.tokenizer.read_token()? {
            Token::Stream => (self.read_word()?, true),
            token => (token.as_word()?, false),
        };

        self.expect_token(Token::CloseParenthesis)?;
        self.expect_token(Token::Returns)?;
        self.expect_token(Token::OpenParenthesis)?;

        let (response_type, response_stream) = match self.tokenizer.read_token()? {
            Token::Stream => (self.read_word()?, true),
            token => (token.as_word()?, false),
        };

        self.expect_token(Token::CloseParenthesis)?;
        self.expect_token(Token::OpenCurlyBracket)?;
        self.tokenizer.skip_until_token(Token::CloseCurlyBracket)?;

        Ok(Rpc::new(
            name,
            request_type,
            request_stream,
            response_type,
            response_stream,
        ))
    }

    fn parse_message_field(
        &mut self,
        type_name: String,
        repeated: bool,
        key_type: Option<String>,
    ) -> Result<(String, Field), ParseError> {
        let field_name = self.read_word()?;
        self.expect_token(Token::Equal)?;
        let field_id = self
            .read_word()?
            .parse::<u32>()
            .map_err(|err| ParseError::ParseFieldId(err))?;

        match self.tokenizer.read_token()? {
            Token::SemiColon => {}
            Token::OpenBracket => {
                self.tokenizer.skip_until_token(Token::CloseBracket)?;
                self.expect_token(Token::SemiColon)?;
            }
            token => return Err(ParseError::UnexpectedToken(token)),
        }

        Ok((
            field_name,
            Field::new(field_id, type_name, repeated, key_type),
        ))
    }

    fn parse_enum(&mut self) -> Result<(String, Vec<EnumTuple>), ParseError> {
        let enum_name = self.read_word()?;
        let mut enum_values = Vec::new();
        self.expect_token(Token::OpenCurlyBracket)?;

        loop {
            match self.tokenizer.read_token()? {
                Token::CloseCurlyBracket => return Ok((enum_name, enum_values)),
                Token::Word(key) => {
                    self.expect_token(Token::Equal)?;
                    let value = self
                        .read_word()?
                        .parse::<u32>()
                        .map_err(|err| ParseError::ParseEnumValue(err))?; // self.expect_token(Token::Word())?;
                    self.expect_token(Token::SemiColon)?;
                    enum_values.push(EnumTuple(key, value));
                }
                token => return Err(ParseError::UnexpectedToken(token)),
            }
        }
    }

    fn parse_message_option(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_reserved(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::SemiColon)?;
        Ok(())
    }

    fn read_word_or_string(&mut self) -> Result<String, ParseError> {
        match self.tokenizer.read_token()? {
            Token::QuotedString(v) => Ok(v),
            Token::Word(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    fn read_quoted_string(&mut self) -> Result<String, ParseError> {
        match self.tokenizer.read_token()? {
            Token::QuotedString(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    fn read_word(&mut self) -> Result<String, ParseError> {
        match self.tokenizer.read_token()? {
            Token::Word(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    fn expect_token(&mut self, expected: Token) -> Result<(), ParseError> {
        let token = self.tokenizer.read_token()?;
        if token == expected {
            return Ok(());
        }
        Err(ParseError::UnexpectedToken(token))
    }

    pub fn print(&self) {
        println!("{}", self.root.fullname)
    }
}

#[cfg(test)]
mod tests {}
