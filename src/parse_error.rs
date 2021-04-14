use crate::{position::Position, token::Token};
use std::{fmt::Display, num::ParseIntError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
#[error("...")]
pub enum TokenError {
    EOF,
    InvalidEscapeSequence(char),
    InvalidStringDelimiter(char),
    MissingEndDelimiter { delimiter: char },
    UnexpectedChar(char),
}

#[derive(Error, Debug, PartialEq)]
#[error("...")]
pub enum ParseError {
    #[error("package not set")]
    PackageNotSet,

    #[error("package already set to")]
    PackageAlreadySet,

    #[error("unexpected top-level token: {0}")]
    UnexpectedTopLevelToken(Token),

    #[error("unexpected message token: {0}")]
    UnexpectedMessageToken(Token),

    #[error("unexpected string: {0}")]
    UnexpectedString(Token),

    #[error("unexpected token: received \"{received}\", expected \"{expected}\"")]
    UnexpectedToken { expected: Token, received: Token },

    #[error("failed to parse field id: {0}")]
    ParseFieldId(ParseIntError),

    #[error("{0}")]
    TokenError(TokenError),
}

impl From<TokenError> for ParseError {
    fn from(error: TokenError) -> Self {
        return ParseError::TokenError(error);
    }
}

#[derive(Debug, PartialEq)]
pub struct ParseFileError<'a> {
    file_name: &'a str,
    content: &'a str,
    position: Position,
    error: ParseError,
}

impl<'a> Display for ParseFileError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line_number = self.position.line;
        let line_number_width = line_number.to_string().len();
        let show_lines = 3;

        let lines = self
            .content
            .split('\n')
            .skip(self.position.line - show_lines)
            .take(show_lines)
            .enumerate()
            .map(|(i, v)| {
                format!(
                    "{:line$} | {}",
                    line_number - (show_lines - i),
                    v,
                    line = line_number_width
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        let padding = (0..self.position.column + line_number_width + 1)
            .map(|_| ' ')
            .collect::<String>();

        write!(f, "{}\n", lines)?;
        write!(f, "{}{}", padding, self.error)
    }
}

impl<'a> ParseFileError<'a> {
    pub fn new(
        file_name: &'a str,
        content: &'a str,
        position: Position,
        error: ParseError,
    ) -> ParseFileError<'a> {
        return Self {
            file_name,
            content,
            position,
            error,
        };
    }
}
