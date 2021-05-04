use std::{collections::HashSet, vec};

use crate::{
    message::{Enum, Field, FieldRule, Message, Oneof},
    namespace::Namespace,
    parse_error::{ParseError, ParseFileError},
    position::Position,
    service::{Rpc, Service},
    token::Token,
    tokenizer::Tokenizer,
};

pub struct Parser {
    pub root: Box<Namespace>,
    parsed_files: HashSet<String>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            root: Namespace::root(),
            parsed_files: HashSet::new(),
        }
    }

    pub fn parse_file<'a>(&mut self, file_name: &'a str) -> Result<(), ParseFileError<'a>> {
        let content = std::fs::read_to_string(file_name).unwrap();
        let mut file_parser = FileParser::new(file_name, &content);
        let namespace = file_parser
            .parse()
            .map_err(|error| (error, file_parser.current_position()))
            .map_err(|it| ParseFileError::new(file_name, content, it.1, it.0))?;

        self.root.append_child(namespace);
        self.parsed_files.insert(file_name.to_string());
        Ok(())
    }

    // fn parse_package(&mut self) -> Result<(), ParseError> {
    //     if self.package.is_some() {
    //         return Err(ParseError::PackageAlreadySet);
    //     }

    //     let name = self.read_word()?;
    //     self.package = self.root.define(name.as_str()).as_ptr();
    //     self.expect_token(Token::SemiColon)?;
    //     Ok(())
    // }

    // pub fn parse(&mut self) -> Result<(), ParseFileError> {
    //     self.parse_internal().map_err(|error| {
    //         ParseFileError::new(
    //             self.file_name,
    //             self.content,
    //             self.tokenizer.current_position(),
    //             error,
    //         )
    //     })?;
    //     Ok(())
    // }
}

pub struct FileParser<'a> {
    pub file_name: &'a str,
    tokenizer: Tokenizer,
    namespace: Option<Box<Namespace>>,
}

impl<'a> FileParser<'a> {
    pub fn new(file_name: &'a str, content: &'a str) -> Self {
        Self {
            file_name,
            tokenizer: Tokenizer::new(content),
            namespace: None,
        }
    }

    pub fn current_position(&self) -> Position {
        return self.tokenizer.current_position();
    }

    pub fn parse(&mut self) -> Result<Box<Namespace>, ParseError> {
        loop {
            match self.tokenizer.next()? {
                Token::EOF => {
                    return self.namespace.take().ok_or(ParseError::PackageNotSet);
                }
                Token::Package => {
                    self.parse_package()?;
                }
                Token::Import => {
                    self.parse_import()?;
                }
                Token::Syntax => {
                    let syntax = self.parse_syntax()?;

                    // TODO handle proto2
                    if syntax == "proto2" {
                        return Err(ParseError::ProtoSyntaxNotSupported(syntax));
                    }
                }
                Token::Option => {
                    self.parse_option()?;
                }
                Token::Service => {
                    let service = self.parse_service()?;
                    self.namespace_mut()?.add_service(service);
                }
                Token::Message => {
                    let (name, message) = self.parse_message()?;
                    self.namespace_mut()?.add_message(name, message);
                }
                Token::Extend => {
                    self.parse_message()?;
                }
                Token::Enum => {
                    let (name, enum_tuples) = self.parse_enum()?;
                    self.namespace_mut()?.add_enum(name, enum_tuples);
                }
                Token::SemiColon => {
                    // relax extra ;
                }

                token => return Err(ParseError::UnexpectedTopLevelToken(token)),
            }
        }
    }

    fn namespace_mut(&mut self) -> Result<&mut Box<Namespace>, ParseError> {
        self.namespace.as_mut().ok_or(ParseError::PackageNotSet)
    }

    fn parse_package(&mut self) -> Result<(), ParseError> {
        if self.namespace.is_some() {
            return Err(ParseError::PackageAlreadySet);
        }

        let name = self.read_word()?;
        self.namespace = Some(Namespace::new(&name, None));

        self.expect_token(Token::SemiColon)?;
        Ok(())
    }

    fn parse_import(&mut self) -> Result<(), ParseError> {
        let import = match self.tokenizer.next()? {
            Token::Public => self.tokenizer.next()?.as_quoted_string()?,
            token => token.as_quoted_string()?,
        };

        self.namespace_mut()?.add_import(import);
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
                    Some((name, oneof)) => message.add_oneof(name, oneof),
                    None => break,
                },
                Token::Message => {
                    let (name, nested_message) = self.parse_message()?;
                    message.add_nested(name, nested_message);
                }
                Token::Oneof => {
                    let name = self.read_word()?;
                    oneof = Some((name, Oneof::new()));
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
                    let (name, field) =
                        self.parse_message_field(type_name, Some(FieldRule::Repeated), None)?;
                    message.add_field(name, field);
                }
                Token::Map => {
                    self.expect_token(Token::OpenAngularBracket)?;
                    let key_type = self.read_word()?;
                    self.expect_token(Token::Comma)?;
                    let type_name = self.read_word()?;
                    self.expect_token(Token::CloseAngularBracket)?;
                    let (name, field) =
                        self.parse_message_field(type_name, None, Some(key_type))?;
                    message.add_field(name, field);
                }
                Token::Word(type_name) => {
                    let (name, field) = self.parse_message_field(type_name, None, None)?;

                    if let Some(ref mut oneof) = oneof {
                        oneof.1.add_field_name(name.to_string())
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
        rule: Option<FieldRule>,
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

        Ok((field_name, Field::new(field_id, type_name, rule, key_type)))
    }

    fn parse_enum(&mut self) -> Result<(String, Enum), ParseError> {
        let enum_name = self.read_word()?;
        let mut e = Enum::new();
        self.expect_token(Token::OpenCurlyBracket)?;

        loop {
            match self.tokenizer.next()? {
                Token::CloseCurlyBracket => return Ok((enum_name, e)),
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

                    e.insert(key, value);
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
}

#[cfg(test)]
mod tests {}
