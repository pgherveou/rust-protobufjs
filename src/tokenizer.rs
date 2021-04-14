use std::vec::IntoIter;

use crate::iterator_with_position::IteratorWithPosition;
use crate::parse_error::TokenError;
use crate::position::Position;
use crate::token::Token;

pub struct Tokenizer {
    chars: IteratorWithPosition<IntoIter<char>>,
    stack: Vec<Token>,
    comment: Option<String>,
}

impl Tokenizer {
    pub fn new(content: &str) -> Self {
        let vec: Vec<char> = content.chars().collect();
        let chars = IteratorWithPosition::new(vec.into_iter());

        Self {
            chars,
            stack: Vec::new(),
            comment: None,
        }
    }

    pub fn current_position(&mut self) -> Position {
        self.chars.current_position()
    }

    pub fn skip_until_token(&mut self, token: Token) -> Result<(), TokenError> {
        loop {
            if self.read_token()? == token {
                return Ok(());
            }
        }
    }

    pub fn read_token(&mut self) -> Result<Token, TokenError> {
        self.next().ok_or(TokenError::EOF)
    }

    fn read_quoted_string(&mut self, delimiter: char) -> Result<String, TokenError> {
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
                    return Err(TokenError::InvalidEscapeSequence(c));
                }
                ('\\', false) => {
                    found_escape_char = true;
                    continue;
                }
                (c, false) if c == delimiter => {
                    found_end_delimiter = true;
                    break;
                }
                (c, false) => vec.push(c),
            }
        }

        if found_end_delimiter {
            Ok(vec.into_iter().collect())
        } else {
            Err(TokenError::MissingEndDelimiter { delimiter })
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
            "package" => Token::Package,
            "reserved" => Token::Reserved,
            "option" => Token::Option,
            "service" => Token::Service,
            "returns" => Token::Returns,
            "rpc" => Token::Rpc,
            "stream" => Token::Stream,
            "repeated" => Token::Repeated,
            "message" => Token::Message,
            "syntax" => Token::Syntax,
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
}

impl Iterator for Tokenizer {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.stack.pop() {
            return Some(token);
        }

        match self.chars.next() {
            None => None,

            Some('=') => Some(Token::Equal),
            Some(';') => Some(Token::SemiColon),
            Some('{') => Some(Token::OpenCurlyBracket),
            Some('}') => Some(Token::CloseCurlyBracket),
            Some('(') => Some(Token::OpenParenthesis),
            Some(')') => Some(Token::CloseParenthesis),

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
                Err(err) => Some(Token::Error(err)),
            },

            // string
            Some(c @ '\'') | Some(c @ '"') => match self.read_quoted_string(c) {
                Ok(str) => {
                    return Some(Token::QuotedString(str));
                }
                Err(err) => Some(Token::Error(err)),
            },

            // word
            Some(c) => Some(self.read_word(c)),
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
            Some(Token::QuotedString("hello world".to_string()))
        );

        assert_eq!(tokenizer.next(), None);
    }

    #[test]
    fn it_should_parse_double_quote_string() {
        let mut tokenizer = Tokenizer::new(r#""hello world""#);

        assert_eq!(
            tokenizer.next(),
            Some(Token::Word("hello world".to_string()))
        );

        assert_eq!(tokenizer.next(), None);
    }

    #[test]
    fn it_should_parse_escaped_string() {
        let mut tokenizer = Tokenizer::new("'hello \\' \n world'");

        assert_eq!(
            tokenizer.next(),
            Some(Token::QuotedString("hello ' \n world".to_string()))
        );

        assert_eq!(tokenizer.next(), None);
    }

    #[test]
    fn it_should_parse_double_slash_comment() {
        let mut tokenizer = Tokenizer::new("// hello world");
        tokenizer.next();
        assert_eq!(tokenizer.comment, Some(" hello world".to_string()));
    }

    #[test]
    fn it_should_parse_slash_star_comment() {
        let mut tokenizer = Tokenizer::new("/* hello world */");
        tokenizer.next();
        assert_eq!(tokenizer.comment, Some(" hello world ".to_string()));
    }
}
