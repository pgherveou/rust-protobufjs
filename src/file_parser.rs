use crate::{
    field::{Field, FieldRule},
    import::Import,
    into_path::IntoPath,
    message::Message,
    metadata::Metadata,
    namespace::Namespace,
    oneof::Oneof,
    parse_error::{ParseError, ParseErrorWithPosition, TokenError},
    r#enum::Enum,
    service::{Rpc, Service},
    token::Token,
    tokenizer::Tokenizer,
};
use std::{path::Path, rc::Rc, vec};

/// FileParser parse a single file into a namespace
pub struct FileParser<I: Iterator> {
    /// The path of the file being parsed. This is used to populate links when generating artifacts
    file_path: Rc<Path>,

    /// The tokenizer used to parse the file
    tokenizer: Tokenizer<I>,

    // Peeked token
    peeked: Option<Result<Token, TokenError>>,

    /// The namespace that will be populated as we parse the file
    namespace: Namespace,
}

impl<I: Iterator<Item = char>> FileParser<I> {
    /// Returns a new parser for the given filename and iterator
    pub fn new(file_path: impl Into<Rc<Path>>, iter: I) -> Self {
        Self {
            file_path: file_path.into(),
            tokenizer: Tokenizer::new(iter),
            peeked: None,
            namespace: Namespace::default(),
        }
    }

    /// Parse the file and return the namespace
    pub fn parse(mut self) -> Result<Namespace, ParseErrorWithPosition> {
        match self.parse_helper() {
            Ok(()) => Ok(self.namespace),
            Err(error) => {
                let position = self.tokenizer.current_position();
                Err(ParseErrorWithPosition(error, position))
            }
        }
    }

    fn parse_helper(&mut self) -> Result<(), ParseError> {
        loop {
            match self.next()? {
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
                    let (name, service) = self.parse_service()?;
                    self.namespace.add_service(name, service);
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
                // relax extra ;
                Token::Semi => {}

                token => return Err(ParseError::UnexpectedTopLevelToken(token)),
            }
        }
    }

    /// Advance the iterator or take the peeked item
    fn next(&mut self) -> Result<Token, TokenError> {
        if let Some(v) = self.peeked.take() {
            return v;
        }

        self.tokenizer.next()
    }

    fn metadata(&mut self) -> Metadata {
        let comment = self.tokenizer.comment.take();
        let line = self.tokenizer.current_line();

        assert!(self.peeked.is_none());

        match comment {
            // get leading_comments if any
            Some(cmt) if cmt.end_line == line - 1 => {
                Metadata::new(self.file_path.clone(), Some(cmt), line)
            }

            // get trailing_comments if any
            _ => {
                // peek next value
                self.peeked.replace(self.tokenizer.next());
                let trailing_comment = match self.tokenizer.comment.as_ref() {
                    Some(cmt) if cmt.start_line == line => self.tokenizer.comment.take(),
                    _ => None,
                };

                Metadata::new(self.file_path.clone(), trailing_comment, line)
            }
        }
    }

    /// Parse the [package] name
    /// For example:
    ///
    /// ```proto
    /// package foo.bar;
    /// ```
    ///
    /// [package] https://developers.google.com/protocol-buffers/docs/proto3#packages
    fn parse_package(&mut self) -> Result<(), ParseError> {
        if !self.namespace.path.is_empty() {
            return Err(ParseError::PackageAlreadySet);
        }

        self.namespace.path = self.read_identifier()?.into_path();
        self.expect_token(Token::Semi)?;
        Ok(())
    }

    /// Parse [import] statement    
    /// For example:
    ///
    /// ```proto
    /// import "myproject/other_protos.proto";
    /// ```
    ///
    /// [import] https://developers.google.com/protocol-buffers/docs/proto3#importing_definitions
    fn parse_import(&mut self) -> Result<(), ParseError> {
        let import = match self.next()? {
            Token::Public => {
                let str = self.next()?.into_quoted_string()?;
                Import::Public(str.into())
            }
            token => {
                let str = token.into_quoted_string()?;
                Import::Internal(str.into())
            }
        };

        self.namespace.add_import(import);
        self.expect_token(Token::Semi)?;
        Ok(())
    }

    /// Parse [syntax] statement
    /// Note: We don't add this information to the namespace,
    /// we only use the result here to validate that the proto syntax is supported     
    ///    
    /// For example:
    ///
    /// ```proto
    /// syntax = "proto3";
    /// ```
    ///
    /// [syntax] https://developers.google.com/protocol-buffers/docs/proto3#simple
    fn parse_syntax(&mut self) -> Result<String, ParseError> {
        self.expect_token(Token::Eq)?;
        let version = self.read_quoted_string()?;
        self.expect_token(Token::Semi)?;
        Ok(version)
    }

    /// Parse [option] statement    
    /// Note: we currently simply parse an option as a list of identifiers
    ///
    /// [option] https://developers.google.com/protocol-buffers/docs/proto3#options
    fn parse_option(&mut self) -> Result<Vec<String>, ParseError> {
        let mut values = Vec::new();
        loop {
            match self.next()? {
                Token::Semi => break,
                Token::EOF => return Err(ParseError::EOF),
                Token::Identifier(s) | Token::String(s) => {
                    values.push(s);
                }
                _ => {}
            }
        }

        Ok(values)
    }

    /// Parse a [message] statement
    ///
    /// For example:
    ///
    /// ```proto
    /// message SearchRequest {
    ///  string query = 1;
    ///  int32 page_number = 2;
    ///  int32 result_per_page = 3;
    /// }
    /// ```
    ///
    /// [message] https://developers.google.com/protocol-buffers/docs/proto3#simple
    fn parse_message(&mut self) -> Result<(String, Message), ParseError> {
        let message_name = self.read_identifier()?;
        self.expect_token(Token::LBrace)?;

        let mut message = Message::new(self.metadata());
        let mut oneof = None;

        loop {
            match self.next()? {
                Token::RBrace => match oneof.take() {
                    Some((name, oneof)) => message.add_oneof(name, oneof),
                    None => break,
                },
                Token::Message => {
                    let (name, nested_message) = self.parse_message()?;
                    message.add_nested_message(name, nested_message);
                }
                Token::Oneof => {
                    let name = self.read_identifier()?;
                    oneof = Some((name, Oneof::new(self.metadata())));
                    self.expect_token(Token::LBrace)?;
                }
                Token::Enum => {
                    let (name, enum_tuples) = self.parse_enum()?;
                    message.add_nested_enum(name, enum_tuples);
                }
                Token::Reserved => {
                    self.parse_reserved()?;
                }
                Token::Extensions => {
                    self.parse_extensions()?;
                }
                Token::Option => {
                    message.md.add_option(self.parse_option()?);
                }
                Token::FieldRule(rule) => {
                    let type_name = self.read_identifier()?;
                    let (name, field) = self.parse_message_field(type_name, Some(rule), None)?;
                    message.add_field(name, field);
                }

                Token::Map => {
                    self.expect_token(Token::LAngle)?;
                    let key_type = self.read_identifier()?;
                    self.expect_token(Token::Comma)?;
                    let type_name = self.read_identifier()?;
                    self.expect_token(Token::Rangle)?;
                    let (name, field) =
                        self.parse_message_field(type_name, None, Some(key_type))?;
                    message.add_field(name, field);
                }
                Token::Identifier(type_name) => {
                    let (name, field) = self.parse_message_field(type_name, None, None)?;

                    if let Some(ref mut oneof) = oneof {
                        oneof.1.add_field_name(name.to_string())
                    }

                    message.add_field(name, field);
                }
                Token::Semi => {
                    // relax extra ";"
                }
                token => return Err(ParseError::UnexpectedMessageToken(token)),
            }
        }

        Ok((message_name, message))
    }

    /// Parse a [service] statement
    /// Returns the name and parsed service object
    /// For example:
    ///
    /// ```proto
    /// service SearchService {
    ///  rpc Search(SearchRequest) returns (SearchResponse);
    /// }
    /// ```
    ///
    /// [service] https://developers.google.com/protocol-buffers/docs/proto3#services
    fn parse_service(&mut self) -> Result<(String, Service), ParseError> {
        let name = self.read_identifier()?;
        let mut service = Service::new(self.metadata());

        self.expect_token(Token::LBrace)?;

        loop {
            match self.next()? {
                Token::RBrace => {
                    break;
                }
                Token::Semi => {
                    // relax extra ;
                }
                Token::Rpc => {
                    let (name, rpc) = self.parse_rpc()?;
                    service.add_rpc(name, rpc)
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

        Ok((name, service))
    }

    /// Parse a [rpc] statement
    /// Returns the rpc name and parsed rpc object
    /// For example:
    ///
    /// ```proto    
    /// rpc Search(SearchRequest) returns (SearchResponse);
    /// ```
    ///
    /// [rpc] https://developers.google.com/protocol-buffers/docs/proto3#services
    fn parse_rpc(&mut self) -> Result<(String, Rpc), ParseError> {
        let name = self.read_identifier()?;
        let mut md = self.metadata();

        self.expect_token(Token::LParen)?;

        let (request_type, request_stream) = match self.next()? {
            Token::Stream => (self.read_identifier()?, true),
            token => (token.identifier()?, false),
        };

        self.expect_token(Token::RParen)?;
        self.expect_token(Token::Returns)?;
        self.expect_token(Token::LParen)?;

        let (response_type, response_stream) = match self.next()? {
            Token::Stream => (self.read_identifier()?, true),
            token => (token.identifier()?, false),
        };

        self.expect_token(Token::RParen)?;

        match self.next()? {
            Token::Semi => {}
            Token::LBrace => loop {
                match self.next()? {
                    Token::Option => {
                        let option = self.parse_option()?;
                        md.add_option(option);
                    }
                    Token::RBrace => {
                        break;
                    }
                    found => {
                        return Err(ParseError::UnexpectedToken {
                            found,
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

        Ok((
            name,
            Rpc::new(
                request_type,
                request_stream,
                response_type,
                response_stream,
                md,
            ),
        ))
    }

    /// Parse a [message] field
    /// Returns the field name and parsed Field object
    /// For example:
    ///
    /// ```proto
    /// string query = 1;
    /// ```
    ///
    /// [message] https://developers.google.com/protocol-buffers/docs/proto3#specifying_field_rules
    fn parse_message_field(
        &mut self,
        type_name: String,
        rule: Option<FieldRule>,
        key_type: Option<String>,
    ) -> Result<(String, Field), ParseError> {
        let field_name = self.read_identifier()?;
        self.expect_token(Token::Eq)?;

        let field_id = self
            .read_identifier()?
            .parse::<u32>()
            .map_err(ParseError::ParseFieldId)?;

        let mut md = self.metadata();
        md.options = vec![self.parse_option()?];

        Ok((
            field_name,
            Field::new(field_id, type_name, rule, key_type, md),
        ))
    }

    /// Parse an [enum]
    /// Returns the enum name and parsed Enum object
    /// For example:
    ///
    /// ```proto
    /// enum Status {
    ///   UNKNOWN = 0;
    ///   STARTED = 1;
    ///   RUNNING = 1;
    /// }
    /// ```
    ///
    /// [enum] https://developers.google.com/protocol-buffers/docs/proto3#enum
    fn parse_enum(&mut self) -> Result<(String, Enum), ParseError> {
        let enum_name = self.read_identifier()?;
        let mut e = Enum::new(self.metadata());
        self.expect_token(Token::LBrace)?;

        loop {
            match self.next()? {
                Token::RBrace => return Ok((enum_name, e)),
                Token::Identifier(key) => {
                    self.expect_token(Token::Eq)?;

                    let val_str = self.read_identifier()?;
                    let val_str_trimmed = val_str.trim_start_matches("0x");
                    let radix = if val_str.eq(val_str_trimmed) { 10 } else { 16 };

                    let value = i32::from_str_radix(val_str_trimmed, radix)
                        .map_err(ParseError::ParseEnumValue)?;

                    match self.next()? {
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
                        expected: vec![Token::RBrace, Token::Identifier("<enum_name>".to_string())],
                    })
                }
            }
        }
    }

    /// Parse a message [reserved] fields
    /// We currently do not parse reserved, we simply fast forward to the end of the statement
    /// For example:
    ///
    /// ```proto
    /// reserved 2, 15, 9 to 11;
    /// ```
    ///
    /// [reserved] https://developers.google.com/protocol-buffers/docs/proto3#reserved
    fn parse_reserved(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::Semi)?;
        Ok(())
    }

    /// Parse a message [extension]
    /// We currently do not parse extensions, we simply fast forward to the end of the statement
    /// For example:
    ///
    /// ```proto
    /// extensions 100 to 199;
    /// ```
    ///
    /// [extension] https://developers.google.com/protocol-buffers/docs/proto#extensions
    fn parse_extensions(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::Semi)?;
        Ok(())
    }

    /// Read a quoted string or fail with an error
    fn read_quoted_string(&mut self) -> Result<String, ParseError> {
        match self.next()? {
            Token::String(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    /// Read a string identifier or fail with an error
    fn read_identifier(&mut self) -> Result<String, ParseError> {
        self.next()?.identifier()
    }

    /// Read the passed token of fail if the next token does not match the expected one
    fn expect_token(&mut self, expected: Token) -> Result<(), ParseError> {
        let token = self.next()?;
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
mod tests {

    use super::FileParser;
    use std::path::PathBuf;

    #[test]
    fn it_should_parse_comment() -> Result<(), Box<dyn std::error::Error>> {
        let file_path: PathBuf = "test.proto".into();
        let text = r#"
        message Foo {
            optional int32 bar = 2; 
            
            // leading comment attached to foo
            optional int32 foo = 1; // trailing comment attached to foo
        }
        "#;

        let parser = FileParser::new(file_path, text.chars());
        let ns = parser.parse()?;
        let cmt = ns
            .types
            .get("Foo")
            .and_then(|t| t.as_message())
            .and_then(|msg| msg.fields.get("foo"))
            .and_then(|f| f.md.comment.as_ref())
            .map(|cmt| cmt.text.as_str());

        println!("{}", cmt.unwrap_or("NONE"));

        Ok(())
    }
    #[test]
    fn playground() -> Result<(), Box<dyn std::error::Error>> {
        let file_path: PathBuf = "test.proto".into();
        let text = r#"
        message Foo {
            option deprecated = true;
            optional int32 foo = 1 [deprecated = true];
        }
        "#;

        let parser = FileParser::new(file_path, text.chars());
        let ns = parser.parse()?;
        let item = ns
            .types
            .get("Foo")
            .and_then(|t| t.as_message())
            .map(|msg| &msg.md);

        println!("{:?}", item);

        Ok(())
    }
}
