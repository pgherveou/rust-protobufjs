use crate::{
    message::{Enum, Field, FieldRule, Message, Oneof},
    namespace::Namespace,
    parse_error::{ParseError, ParseFileError},
    service::{Rpc, Service},
    token::Token,
    tokenizer::Tokenizer,
};
use std::{collections::HashSet, path::PathBuf, vec};

/// The parser parse files and populate the root namespace
pub struct Parser {
    /// The root namespace that will receive child namespace parsed in each file
    pub root: Box<Namespace>,

    /// The root directory used to resolve import statements
    root_dir: PathBuf,

    /// List of parsed files, used to do some bookeeping when resolving imports
    parsed_files: HashSet<PathBuf>,
}

impl Parser {
    /// Returns a new parser with the given root directory and a list of files we want to ignore    
    pub fn new(root_dir: PathBuf, ignored_files: HashSet<PathBuf>) -> Self {
        Self {
            root_dir,
            root: Namespace::root(),
            // populate the parsed_files with the passed ignored_files to treat them as already parsed
            parsed_files: ignored_files,
        }
    }

    /// Parse the given file, and it's import dependencies
    /// The result will be merged into the root namespace of the parser
    pub fn parse_file(&mut self, file_name: PathBuf) -> Result<(), ParseFileError> {
        if self.parsed_files.contains(&file_name) {
            return Ok(());
        }

        self.parsed_files.insert(file_name.clone());

        let content = match std::fs::read_to_string(&file_name) {
            Ok(r) => r,
            Err(error) => return Err(ParseFileError::Read(file_name, error)),
        };

        let file_parser = FileParser::new(file_name, content.chars());
        let namespace = file_parser.parse(&content)?;

        for file in namespace.imports.iter() {
            let import_path = self.root_dir.join(file);
            self.parse_file(import_path)?;
        }

        self.root.append_child(namespace);
        Ok(())
    }
}

/// FileParser parse a single file into a namespace
pub struct FileParser<I: Iterator> {
    /// The path of the file being parsed. This will be used to populate links when generating artifacts
    file_name: PathBuf,

    /// The tokenizer used to parse the file
    tokenizer: Tokenizer<I>,

    /// the namespace that will be populated as we parse the file
    namespace: Box<Namespace>,
}

impl<I: Iterator<Item = char>> FileParser<I> {
    /// Returns a new parser for the given filename and iterator
    pub fn new(file_name: PathBuf, iter: I) -> Self {
        Self {
            file_name,
            tokenizer: Tokenizer::new(iter),
            namespace: Namespace::root(),
        }
    }

    /// Parse the file and return the namespace
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

    /// Parse the file
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

    /// Parse the [package] name
    /// For example:
    ///
    /// ```proto
    /// package foo.bar;
    /// ```
    ///
    /// [package] https://developers.google.com/protocol-buffers/docs/proto3#packages
    fn parse_package(&mut self) -> Result<(), ParseError> {
        if !self.namespace.fullname.is_empty() {
            return Err(ParseError::PackageAlreadySet);
        }

        self.namespace.fullname = self.read_identifier()?;
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
        let import = match self.tokenizer.next()? {
            Token::Public => self.tokenizer.next()?.as_quoted_string()?,
            token => token.as_quoted_string()?,
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
    /// Note we currently don't use the options,
    /// the parser simply fast forward until the end of the option declaration
    ///
    /// [option] https://developers.google.com/protocol-buffers/docs/proto3#options
    fn parse_option(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::Semi)?;
        Ok(())
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
                    message.add_nested_message(name, nested_message);
                }
                Token::Oneof => {
                    let name = self.read_identifier()?;
                    oneof = Some((name, Oneof::new()));
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
                    self.parse_message_option()?;
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
        let mut service = Service::new();

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

        self.expect_token(Token::LParen)?;

        let (request_type, request_stream) = match self.tokenizer.next()? {
            Token::Stream => (self.read_identifier()?, true),
            token => (token.identifier()?, false),
        };

        self.expect_token(Token::RParen)?;
        self.expect_token(Token::Returns)?;
        self.expect_token(Token::LParen)?;

        let (response_type, response_stream) = match self.tokenizer.next()? {
            Token::Stream => (self.read_identifier()?, true),
            token => (token.identifier()?, false),
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

        Ok((
            name,
            Rpc::new(request_type, request_stream, response_type, response_stream),
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

        // usize::from_str_radix(num.trim_start_matches("0x"), 16)

        let field_id = self
            .read_identifier()?
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
        let mut e = Enum::new();
        self.expect_token(Token::LBrace)?;

        loop {
            match self.tokenizer.next()? {
                Token::RBrace => return Ok((enum_name, e)),
                Token::Identifier(key) => {
                    self.expect_token(Token::Eq)?;

                    let val_str = self.read_identifier()?;
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
                        expected: vec![Token::RBrace, Token::Identifier("<enum_name>".to_string())],
                    })
                }
            }
        }
    }

    /// Parse a message options
    /// We currently do not parse options, we simply fast forward to the end of the statement
    /// For example:
    ///
    /// ```proto
    /// option deprecated = true;
    /// ```
    fn parse_message_option(&mut self) -> Result<(), ParseError> {
        self.tokenizer.skip_until_token(Token::Semi)?;
        Ok(())
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
        match self.tokenizer.next()? {
            Token::String(v) => Ok(v),
            token => Err(ParseError::UnexpectedString(token)),
        }
    }

    /// Read a string identifier or fail with an error
    fn read_identifier(&mut self) -> Result<String, ParseError> {
        self.tokenizer.next()?.identifier()
    }

    /// Read the passed token of fail if the next token is not equal
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
