use crate::{position::Position, token::Token};
use std::{io, num::ParseIntError, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
#[error("...")]
pub enum TokenError {
    #[error("Unexpected end of file")]
    EOF,

    #[error("Invalid delimiter {0}")]
    InvalidStringDelimiter(char),

    #[error("Invalid end delimiter {0}")]
    MissingEndDelimiter(char),

    #[error("Unexpected char {0}")]
    UnexpectedChar(char),
}

#[derive(Error, Debug, PartialEq)]
#[error("...")]
pub enum ParseError {
    #[error("package not set")]
    PackageNotSet,

    #[error("proto version {0} not supported")]
    ProtoSyntaxNotSupported(String),

    #[error("package already set to")]
    PackageAlreadySet,

    #[error("unexpected top-level token: {0}")]
    UnexpectedTopLevelToken(Token),

    #[error("unexpected string: {0}")]
    UnexpectedString(Token),

    #[error("unexpected token: {0}")]
    IllegalToken(Token),

    #[error("unexpected token: \"{found}\" expected one of {expected:?}")]
    UnexpectedToken { found: Token, expected: Vec<Token> },

    #[error("failed to parse field id: {0}")]
    ParseFieldId(ParseIntError),

    #[error("failed to parse enum value: {0}")]
    ParseEnumValue(ParseIntError),

    #[error("{0}")]
    TokenError(TokenError),
}

impl From<TokenError> for ParseError {
    fn from(error: TokenError) -> Self {
        return ParseError::TokenError(error);
    }
}

#[derive(Error, Debug)]
#[error("...")]
pub enum ParseFileError {
    #[error("Failed to read file {file_name}. {error}")]
    Read {
        file_name: PathBuf,
        error: io::Error,
    },

    #[error("{0}")]
    ParseError(String),
}

impl ParseFileError {
    pub fn from_parse_error(
        error: ParseError,
        file_name: PathBuf,
        content: &str,
        position: Position,
    ) -> ParseFileError {
        let line_number = position.line;
        let line_number_width = line_number.to_string().len();
        let show_lines = std::cmp::min(position.line, 3);

        let lines = content
            .split('\n')
            .skip(position.line - show_lines)
            .take(show_lines)
            .enumerate()
            .map(|(i, v)| {
                format!(
                    "{:line$} | {}",
                    line_number - (show_lines - i - 1),
                    v,
                    line = line_number_width
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        let padding = (0..position.column + line_number_width + 1)
            .map(|_| ' ')
            .collect::<String>();

        ParseFileError::ParseError(format!(
            "Failed to parse {}\n{}\n{}{}",
            file_name.display(),
            lines,
            padding,
            error
        ))
    }
}
