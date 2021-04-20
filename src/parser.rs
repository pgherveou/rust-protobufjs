use std::{ptr::NonNull, vec};

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
        loop {
            match self.tokenizer.next()? {
                Token::EOF => {
                    return Ok(());
                }
                Token::Package => {
                    self.parse_package()?;
                }
                Token::Import => {
                    self.parse_import()?;
                }
                Token::Syntax => {
                    let syntax = self.parse_syntax()?;

                    // ignore proto2 for now
                    if syntax == "proto2" {
                        return Ok(());
                    }
                }
                Token::Option => {
                    self.parse_option()?;
                }
                Token::Service => {
                    let service = self.parse_service()?;
                    self.package_mut()?.add_service(service);
                }
                Token::Message => {
                    let (name, message) = self.parse_message()?;
                    self.package_mut()?.add_message(name, message);
                }
                Token::Extend => {
                    self.parse_message()?;
                }
                Token::Enum => {
                    let (name, enum_tuples) = self.parse_enum()?;
                    self.package_mut()?.add_enum(name, enum_tuples);
                }
                Token::SemiColon => {
                    // relax extra ;
                }

                token => return Err(ParseError::UnexpectedTopLevelToken(token)),
            }
        }
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
        match self.tokenizer.next()? {
            Token::Public => {
                self.tokenizer.next()?.as_quoted_string()?;
            }
            token => {
                token.as_quoted_string()?;
            }
        }

        self.expect_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_syntax(&mut self) -> Result<String, ParseError> {
        self.expect_token(Token::Equal)?;
        let version = self.read_quoted_string()?;
        self.expect_token(Token::SemiColon)?;
        Ok(version)
    }

    fn parse_option(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_message(&mut self) -> Result<(String, Message), ParseError> {
        let message_name = self.read_word()?;
        self.expect_token(Token::OpenCurlyBracket)?;

        let mut message = Message::new();
        let mut oneof = None;

        loop {
            match self.tokenizer.next()? {
                Token::CloseCurlyBracket => match oneof.take() {
                    Some(oneof) => message.add_oneof(oneof),
                    None => break,
                },
                Token::Message => {
                    let (name, nested_message) = self.parse_message()?;
                    message.add_nested(name, nested_message);
                    // self.expect_token(Token::SemiColon)?;
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
                Token::SemiColon => {
                    // relax extra ";"
                }
                token => return Err(ParseError::IllegalToken(token)),
            }
        }

        Ok((message_name, message))
    }

    fn parse_service(&mut self) -> Result<Service, ParseError> {
        let name = self.read_word()?;
        let mut service = Service::new(name);

        self.expect_token(Token::OpenCurlyBracket)?;

        loop {
            match self.tokenizer.next()? {
                Token::CloseCurlyBracket => {
                    break;
                }
                Token::SemiColon => {
                    // relax extra ;
                }
                Token::Rpc => {
                    let rpc = self.parse_rpc()?;
                    service.add_rpc(rpc)
                }
                Token::Option => {
                    self.parse_option()?;
                }
                found => {
                    return Err(ParseError::UnexpectedToken {
                        found,
                        expected: vec![Token::CloseCurlyBracket, Token::Rpc, Token::Option],
                    })
                }
            }
        }

        Ok(service)
    }

    fn parse_rpc(&mut self) -> Result<Rpc, ParseError> {
        let name = self.read_word()?;

        self.expect_token(Token::OpenParenthesis)?;

        let (request_type, request_stream) = match self.tokenizer.next()? {
            Token::Stream => (self.read_word()?, true),
            token => (token.as_word()?, false),
        };

        self.expect_token(Token::CloseParenthesis)?;
        self.expect_token(Token::Returns)?;
        self.expect_token(Token::OpenParenthesis)?;

        let (response_type, response_stream) = match self.tokenizer.next()? {
            Token::Stream => (self.read_word()?, true),
            token => (token.as_word()?, false),
        };

        self.expect_token(Token::CloseParenthesis)?;

        match self.tokenizer.next()? {
            Token::SemiColon => {}
            Token::OpenCurlyBracket => loop {
                match self.tokenizer.next()? {
                    Token::Option => {
                        self.parse_option()?;
                    }
                    Token::CloseCurlyBracket => {
                        break;
                    }
                    found => {
                        return Err(ParseError::UnexpectedToken {
                            found: found,
                            expected: vec![Token::Option, Token::CloseCurlyBracket],
                        })
                    }
                }
            },
            found => {
                return Err(ParseError::UnexpectedToken {
                    found,
                    expected: vec![Token::SemiColon, Token::OpenCurlyBracket],
                })
            }
        }

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

        // usize::from_str_radix(num.trim_start_matches("0x"), 16)

        let field_id = self
            .read_word()?
            .parse::<u32>()
            .map_err(|err| ParseError::ParseFieldId(err))?;

        match self.tokenizer.next()? {
            Token::SemiColon => {}
            Token::OpenBracket => {
                self.tokenizer.skip_until_token(Token::SemiColon)?;
            }
            found => {
                return Err(ParseError::UnexpectedToken {
                    found,
                    expected: vec![Token::SemiColon, Token::OpenBracket],
                })
            }
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
            match self.tokenizer.next()? {
                Token::CloseCurlyBracket => return Ok((enum_name, enum_values)),
                Token::Word(key) => {
                    self.expect_token(Token::Equal)?;

                    let val_str = self.read_word()?;
                    let val_str_trimmed = val_str.trim_start_matches("0x");
                    let radix = if val_str.eq(val_str_trimmed) { 10 } else { 16 };

                    let value = i32::from_str_radix(val_str_trimmed, radix)
                        .map_err(|err| ParseError::ParseEnumValue(err))?;

                    match self.tokenizer.next()? {
                        Token::SemiColon => {}
                        Token::OpenBracket => {
                            self.tokenizer.skip_until_token(Token::CloseBracket)?;
                            self.expect_token(Token::SemiColon)?;
                        }
                        found => {
                            return Err(ParseError::UnexpectedToken {
                                found,
                                expected: vec![Token::SemiColon, Token::OpenBracket],
                            })
                        }
                    }

                    enum_values.push(EnumTuple(key, value));
                }
                Token::Option => {
                    self.parse_option()?;
                }
                Token::Reserved => {
                    self.tokenizer.skip_until_token(Token::SemiColon)?;
                }
                found => {
                    return Err(ParseError::UnexpectedToken {
                        found,
                        expected: vec![
                            Token::CloseCurlyBracket,
                            Token::Word("<enum_name>".to_string()),
                        ],
                    })
                }
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

    fn read_quoted_string(&mut self) -> Result<String, ParseError> {
        match self.tokenizer.next()? {
            Token::QuotedString(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    fn read_word(&mut self) -> Result<String, ParseError> {
        self.tokenizer.next()?.as_word()
    }

    fn expect_token(&mut self, expected: Token) -> Result<(), ParseError> {
        let token = self.tokenizer.next()?;
        if token == expected {
            return Ok(());
        }
        Err(ParseError::UnexpectedToken {
            found: token,
            expected: vec![expected],
        })
    }

    pub fn print(&self) {
        println!("{}", self.root.fullname)
    }
}

#[cfg(test)]
mod tests {}
