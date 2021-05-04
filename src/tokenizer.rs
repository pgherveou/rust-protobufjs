use std::vec::IntoIter;

use crate::iterator_with_position::IteratorWithPosition;
use crate::parse_error::TokenError;
use crate::position::Position;
use crate::token::Token;

pub struct Tokenizer {
    chars: IteratorWithPosition<IntoIter<char>>,
    comment: Option<String>,
}

impl Tokenizer {
    pub fn new(content: &str) -> Self {
        let vec: Vec<char> = content.chars().collect();
        let chars = IteratorWithPosition::new(vec.into_iter());

        Self {
            chars,
            comment: None,
        }
    }

    pub fn current_position(&self) -> Position {
        self.chars.current_position()
    }

    pub fn skip_until_token(&mut self, token: Token) -> Result<(), TokenError> {
        loop {
            if self.next()? == token {
                return Ok(());
            }
        }
    }

    fn read_delimited_string(&mut self, end_delimiter: char) -> Result<String, TokenError> {
        let mut vec = Vec::new();
        let mut found_escape_char = false;
        let mut found_end_delimiter = false;

        // quick macro used to avoid repetition in the match branches below
        macro_rules! push_and_reset {
            ($x:expr) => {{
                vec.push($x);
                found_escape_char = false;
            }};
        }

        while let Some(char) = self.chars.next() {
            match (char, found_escape_char) {
                ('n', true) => push_and_reset!('\n'),
                ('r', true) => push_and_reset!('\r'),
                ('t', true) => push_and_reset!('\t'),
                ('\\', true) => push_and_reset!('\\'),
                ('"', true) => push_and_reset!('\"'),
                ('\'', true) => push_and_reset!('\''),
                (c, true) => {
                    vec.push('\\');
                    push_and_reset!(c)
                }
                ('\\', false) => {
                    vec.push('\\');
                    found_escape_char = true;
                    continue;
                }
                (c, false) if c == end_delimiter => {
                    found_end_delimiter = true;
                    break;
                }
                (c, false) => vec.push(c),
            }
        }

        if found_end_delimiter {
            Ok(vec.into_iter().collect())
        } else {
            Err(TokenError::MissingEndDelimiter(end_delimiter))
        }
    }

    fn read_word(&mut self, start: char) -> Token {
        let mut vec = vec![start];

        while let Some(char) = self.chars.next_if(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' => true,
            _ => false,
        }) {
            vec.push(char);
        }

        let word = vec.into_iter().collect::<String>();
        match word.as_str() {
            "import" => Token::Import,
            "public" => Token::Public,
            "package" => Token::Package,
            "reserved" => Token::Reserved,
            "option" => Token::Option,
            "service" => Token::Service,
            "returns" => Token::Returns,
            "rpc" => Token::Rpc,
            "stream" => Token::Stream,
            "repeated" => Token::Repeated,
            "map" => Token::Map,
            "message" => Token::Message,
            "extend" => Token::Extend,
            "syntax" => Token::Syntax,
            "oneof" => Token::Oneof,
            "enum" => Token::Enum,
            _ => Token::Word(word),
        }
    }

    fn read_comment(&mut self) -> Result<String, TokenError> {
        let char = self.chars.next().unwrap_or(' ');

        match char {
            // /* slash star comment */
            '*' => {
                let mut previous_char = self.chars.next().unwrap_or(' ');
                let mut comment = String::new();

                while let Some(current_char) = self.chars.next() {
                    match (previous_char, current_char) {
                        ('*', '/') => return Ok(comment),
                        _ => {
                            comment.push(previous_char);
                            previous_char = current_char
                        }
                    }
                }

                // TODO Trim
                return Ok(comment);
            }

            // // double slash comment
            '/' => {
                let mut comment = String::new();
                while let Some(c) = self.chars.next_if(|c| *c != '\n') {
                    comment.push(c);
                }

                // TODO cleanup triple slash
                return Ok(comment);
            }

            found => return Err(TokenError::UnexpectedChar(found)),
        }
    }

    pub fn next(&mut self) -> Result<Token, TokenError> {
        match self.chars.next() {
            None => Ok(Token::EOF),

            Some('=') => Ok(Token::Equal),
            Some(';') => Ok(Token::SemiColon),
            Some('{') => Ok(Token::OpenCurlyBracket),
            Some('}') => Ok(Token::CloseCurlyBracket),
            Some('(') => Ok(Token::OpenParenthesis),
            Some(')') => Ok(Token::CloseParenthesis),
            Some('[') => Ok(Token::OpenBracket),
            Some(']') => Ok(Token::CloseBracket),
            Some('<') => Ok(Token::OpenAngularBracket),
            Some('>') => Ok(Token::CloseAngularBracket),
            Some(',') => Ok(Token::Comma),

            // whitespaces
            Some(' ') | Some('\t') => self.next(),

            // New line
            Some('\n') | Some('\r') => {
                self.comment = None;
                self.next()
            }

            // comment
            Some('/') => match self.read_comment() {
                Ok(comment) => {
                    self.comment = Some(comment);
                    self.next()
                }
                Err(err) => Err(err),
            },

            // Quoted string
            Some(c @ '\'') | Some(c @ '"') => match self.read_delimited_string(c) {
                Ok(str) => {
                    return Ok(Token::QuotedString(str));
                }
                Err(err) => Err(err),
            },

            // word
            Some(c) => Ok(self.read_word(c)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::token::Token;
    use crate::tokenizer::Tokenizer;

    #[test]
    fn it_should_parse_single_quote_string() {
        let mut tokenizer = Tokenizer::new("'hello world'");

        assert_eq!(
            tokenizer.next(),
            Ok(Token::QuotedString("hello world".to_string()))
        );
    }

    #[test]
    fn it_should_parse_double_quote_string() {
        let mut tokenizer = Tokenizer::new(r#""hello world""#);

        assert_eq!(tokenizer.next(), Ok(Token::Word("hello world".to_string())));
    }

    #[test]
    fn it_should_parse_escaped_string() {
        let mut tokenizer = Tokenizer::new("'hello \\' \n world'");

        assert_eq!(
            tokenizer.next(),
            Ok(Token::QuotedString("hello ' \n world".to_string()))
        );
    }

    #[test]
    fn it_should_parse_double_slash_comment() {
        let mut tokenizer = Tokenizer::new("// hello world");
        tokenizer.next().unwrap();
        assert_eq!(tokenizer.comment, Some(" hello world".to_string()));
    }

    #[test]
    fn it_should_parse_slash_star_comment() {
        let mut tokenizer = Tokenizer::new("/* hello world */");
        tokenizer.next().unwrap();
        assert_eq!(tokenizer.comment, Some(" hello world ".to_string()));
    }
}
