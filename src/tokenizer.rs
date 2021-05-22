use crate::parse_error::TokenError;
use crate::position::Position;
use crate::token::Token;
use crate::{field::FieldRule, iterator_with_position::IteratorWithPosition};

/// A tokenizer reads from the `chars` iterator and produce `Token`
pub struct Tokenizer<I: Iterator> {
    /// The chars iterators
    chars: IteratorWithPosition<I>,

    /// The current comment if any
    pub comment: Option<String>,
}

impl<I: Iterator<Item = char>> Tokenizer<I> {
    /// Returns a new Tokenizer for the given char iterator
    pub fn new(chars: I) -> Self {
        Self {
            chars: IteratorWithPosition::new(chars),
            comment: None,
        }
    }

    /// Returns the current position
    pub fn current_position(&self) -> Position {
        self.chars.current_position()
    }

    /// Skip tokens until it matches the passed token
    pub fn skip_until_token(&mut self, token: Token) -> Result<(), TokenError> {
        loop {
            if self.next()? == token {
                return Ok(());
            }
        }
    }

    /// Return the string delimited by the specified char
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

    /// Return the next identifier starting with given char
    fn read_identifier(&mut self, start: char) -> Token {
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
            "extensions" => Token::Extensions,
            "repeated" => Token::FieldRule(FieldRule::Repeated),
            "optional" => Token::FieldRule(FieldRule::Optional),
            "required" => Token::FieldRule(FieldRule::Required),
            "map" => Token::Map,
            "message" => Token::Message,
            "extend" => Token::Extend,
            "syntax" => Token::Syntax,
            "oneof" => Token::Oneof,
            "enum" => Token::Enum,
            _ => Token::Identifier(word),
        }
    }

    /// Return the next comment
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

    /// Returns the next token
    pub fn next(&mut self) -> Result<Token, TokenError> {
        match self.chars.next() {
            None => Ok(Token::EOF),

            Some('=') => Ok(Token::Eq),
            Some(';') => Ok(Token::Semi),
            Some(':') => Ok(Token::Colon),
            Some('{') => Ok(Token::LBrace),
            Some('}') => Ok(Token::RBrace),
            Some('(') => Ok(Token::LParen),
            Some(')') => Ok(Token::RParen),
            Some('[') => Ok(Token::LBrack),
            Some(']') => Ok(Token::RBrack),
            Some('<') => Ok(Token::LAngle),
            Some('>') => Ok(Token::Rangle),
            Some(',') => Ok(Token::Comma),

            // whitespaces
            Some(' ') | Some('\t') => self.next(),

            // New line
            Some('\n') | Some('\r') => {
                self.comment = None;
                self.next()
            }

            // comment
            Some('/') => {
                self.comment = Some(self.read_comment()?);
                self.next()
            }

            // Quoted string
            Some(c @ '\'') | Some(c @ '"') => Ok(Token::String(self.read_delimited_string(c)?)),

            // word
            Some(c) => Ok(self.read_identifier(c)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tokenizer::Tokenizer;
    use crate::{parse_error::TokenError, token::Token};

    #[test]
    fn it_should_parse_single_quote_string() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new("'hello world'".chars());
        assert_eq!(tokenizer.next()?, Token::String("hello world".to_string()));
        Ok(())
    }

    #[test]
    fn it_should_parse_double_quote_string() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new(r#""hello world""#.chars());
        assert_eq!(
            tokenizer.next()?,
            Token::Identifier("hello world".to_string())
        );
        Ok(())
    }

    #[test]
    fn it_should_parse_escaped_string() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new("'hello \\' \n world'".chars());
        assert_eq!(
            tokenizer.next()?,
            Token::String("hello ' \n world".to_string())
        );
        Ok(())
    }

    #[test]
    fn it_should_parse_double_slash_comment() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new("// hello world".chars());
        tokenizer.next()?;
        assert_eq!(tokenizer.comment, Some(" hello world".to_string()));
        Ok(())
    }

    #[test]
    fn it_should_parse_slash_star_comment() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new("/* hello world */".chars());
        tokenizer.next()?;
        assert_eq!(tokenizer.comment, Some(" hello world ".to_string()));
        Ok(())
    }
}
