use std::{collections::HashSet, path::PathBuf, vec};

use crate::{
    message::{Enum, Field, FieldRule, Message, Oneof},
    namespace::Namespace,
    parse_error::{ParseError, ParseFileError},
    service::{Rpc, Service},
    token::Token,
    tokenizer::Tokenizer,
};

pub struct Parser {
    pub root: Box<Namespace>,
    root_dir: PathBuf,
    parsed_files: HashSet<PathBuf>,
}

impl Parser {
    pub fn new(root_dir: PathBuf, ignored_files: HashSet<PathBuf>) -> Self {
        Self {
            root: Namespace::root(),
            parsed_files: ignored_files,
            root_dir,
        }
    }

    pub fn parse_file(&mut self, file_name: PathBuf) -> Result<(), ParseFileError> {
        if self.parsed_files.contains(&file_name) {
            return Ok(());
        }

        self.parsed_files.insert(file_name.clone());
        let content = match std::fs::read_to_string(&file_name) {
            Ok(r) => r,
            Err(error) => return Err(ParseFileError::Read { file_name, error }),
        };

        let iter = content.chars();
        let file_parser = FileParser::new(file_name, iter);

        let namespace = file_parser.parse(&content)?;
        for file in namespace.imports.iter() {
            let import_path = self.root_dir.join(file);
            self.parse_file(import_path)?;
        }

        self.root.append_child(namespace);
        Ok(())
    }
}

pub struct FileParser<I: Iterator> {
    pub file_name: PathBuf,
    tokenizer: Tokenizer<I>,
    namespace: Box<Namespace>,
}

impl<I: Iterator<Item = char>> FileParser<I> {
    pub fn new(file_name: PathBuf, iter: I) -> Self {
        Self {
            file_name,
            tokenizer: Tokenizer::new(iter),
            namespace: Namespace::root(),
        }
    }

    pub fn parse(mut self, content: &str) -> Result<Box<Namespace>, ParseFileError> {
        match self.parse_helper() {
            Ok(()) => Ok(self.namespace),
            Err(error) => {
                let position = self.tokenizer.current_position();
                let Self { file_name, .. } = self;
                Err(ParseFileError::from_parse_error(
                    error, file_name, content, position,
                ))
            }
        }
    }

    fn parse_helper(&mut self) -> Result<(), ParseError> {
        loop {
            match self.tokenizer.next()? {
                Token::EOF => return Ok(()),
                Token::Package => {
                    self.parse_package()?;
                }
                Token::Import => {
                    self.parse_import()?;
                }
                Token::Syntax => {
                    let syntax = self.parse_syntax()?;
                    if syntax != "proto3" && syntax != "proto2" {
                        return Err(ParseError::ProtoSyntaxNotSupported(syntax));
                    }
                }
                Token::Option => {
                    self.parse_option()?;
                }
                Token::Service => {
                    let service = self.parse_service()?;
                    self.namespace.add_service(service);
                }
                Token::Message => {
                    let (name, message) = self.parse_message()?;
                    self.namespace.add_message(name, message);
                }
                Token::Extend => {
                    self.parse_message()?;
                }
                Token::Enum => {
                    let (name, enum_tuples) = self.parse_enum()?;
                    self.namespace.add_enum(name, enum_tuples);
                }
                Token::Semi => {
                    // relax extra ;
                }

                token => return Err(ParseError::UnexpectedTopLevelToken(token)),
            }
        }
    }

    fn parse_package(&mut self) -> Result<(), ParseError> {
        if !self.namespace.fullname.is_empty() {
            return Err(ParseError::PackageAlreadySet);
        }

        self.namespace.fullname = self.read_word()?;
        self.expect_token(Token::Semi)?;
        Ok(())
    }

    fn parse_import(&mut self) -> Result<(), ParseError> {
        let import = match self.tokenizer.next()? {
            Token::Public => self.tokenizer.next()?.as_quoted_string()?,
            token => token.as_quoted_string()?,
        };

        self.namespace.add_import(import);
        self.expect_token(Token::Semi)?;
        Ok(())
    }

    fn parse_syntax(&mut self) -> Result<String, ParseError> {
        self.expect_token(Token::Equal)?;
        let version = self.read_quoted_string()?;
        self.expect_token(Token::Semi)?;
        Ok(version)
    }

    fn parse_option(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::Semi)?;
        Ok(())
    }

    fn parse_message(&mut self) -> Result<(String, Message), ParseError> {
        let message_name = self.read_word()?;
        self.expect_token(Token::LBrace)?;

        let mut message = Message::new();
        let mut oneof = None;

        loop {
            match self.tokenizer.next()? {
                Token::RBrace => match oneof.take() {
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
                    self.expect_token(Token::LBrace)?;
                }
                Token::Enum => {
                    let (name, enum_tuples) = self.parse_enum()?;
                    message.add_enum(name, enum_tuples);
                }
                Token::Reserved => {
                    self.parse_reserved()?;
                }
                Token::Extensions => {
                    self.parse_extensions()?;
                }
                Token::Option => {
                    self.parse_message_option()?;
                }
                Token::FieldRule(rule) => {
                    let type_name = self.read_word()?;
                    let (name, field) = self.parse_message_field(type_name, Some(rule), None)?;
                    message.add_field(name, field);
                }

                Token::Map => {
                    self.expect_token(Token::LAngle)?;
                    let key_type = self.read_word()?;
                    self.expect_token(Token::Comma)?;
                    let type_name = self.read_word()?;
                    self.expect_token(Token::Rangle)?;
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
                Token::Semi => {
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

        self.expect_token(Token::LBrace)?;

        loop {
            match self.tokenizer.next()? {
                Token::RBrace => {
                    break;
                }
                Token::Semi => {
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
                        expected: vec![Token::RBrace, Token::Rpc, Token::Option],
                    })
                }
            }
        }

        Ok(service)
    }

    fn parse_rpc(&mut self) -> Result<Rpc, ParseError> {
        let name = self.read_word()?;

        self.expect_token(Token::LParen)?;

        let (request_type, request_stream) = match self.tokenizer.next()? {
            Token::Stream => (self.read_word()?, true),
            token => (token.as_word()?, false),
        };

        self.expect_token(Token::RParen)?;
        self.expect_token(Token::Returns)?;
        self.expect_token(Token::LParen)?;

        let (response_type, response_stream) = match self.tokenizer.next()? {
            Token::Stream => (self.read_word()?, true),
            token => (token.as_word()?, false),
        };

        self.expect_token(Token::RParen)?;

        match self.tokenizer.next()? {
            Token::Semi => {}
            Token::LBrace => loop {
                match self.tokenizer.next()? {
                    Token::Option => {
                        self.parse_option()?;
                    }
                    Token::RBrace => {
                        break;
                    }
                    found => {
                        return Err(ParseError::UnexpectedToken {
                            found: found,
                            expected: vec![Token::Option, Token::RBrace],
                        })
                    }
                }
            },
            found => {
                return Err(ParseError::UnexpectedToken {
                    found,
                    expected: vec![Token::Semi, Token::LBrace],
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
            Token::Semi => {}
            Token::LBrack => {
                self.tokenizer.skip_until_token(Token::Semi)?;
            }
            found => {
                return Err(ParseError::UnexpectedToken {
                    found,
                    expected: vec![Token::Semi, Token::LBrack],
                })
            }
        }

        Ok((field_name, Field::new(field_id, type_name, rule, key_type)))
    }

    fn parse_enum(&mut self) -> Result<(String, Enum), ParseError> {
        let enum_name = self.read_word()?;
        let mut e = Enum::new();
        self.expect_token(Token::LBrace)?;

        loop {
            match self.tokenizer.next()? {
                Token::RBrace => return Ok((enum_name, e)),
                Token::Word(key) => {
                    self.expect_token(Token::Equal)?;

                    let val_str = self.read_word()?;
                    let val_str_trimmed = val_str.trim_start_matches("0x");
                    let radix = if val_str.eq(val_str_trimmed) { 10 } else { 16 };

                    let value = i32::from_str_radix(val_str_trimmed, radix)
                        .map_err(|err| ParseError::ParseEnumValue(err))?;

                    match self.tokenizer.next()? {
                        Token::Semi => {}
                        Token::LBrack => {
                            self.tokenizer.skip_until_token(Token::RBrack)?;
                            self.expect_token(Token::Semi)?;
                        }
                        found => {
                            return Err(ParseError::UnexpectedToken {
                                found,
                                expected: vec![Token::Semi, Token::LBrack],
                            })
                        }
                    }

                    e.insert(key, value);
                }
                Token::Option => {
                    self.parse_option()?;
                }
                Token::Reserved => {
                    self.tokenizer.skip_until_token(Token::Semi)?;
                }
                found => {
                    return Err(ParseError::UnexpectedToken {
                        found,
                        expected: vec![Token::RBrace, Token::Word("<enum_name>".to_string())],
                    })
                }
            }
        }
    }

    fn parse_message_option(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::Semi)?;
        Ok(())
    }

    fn parse_reserved(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::Semi)?;
        Ok(())
    }

    fn parse_extensions(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::Semi)?;
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
