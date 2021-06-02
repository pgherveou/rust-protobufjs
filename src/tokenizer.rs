use crate::comment::Comment;
use crate::comment::CommentKind;
use crate::parse_error::TokenError;
use crate::position::Position;
use crate::token::Token;
use crate::{field::FieldRule, iterator_with_position::IteratorWithPosition};

/// A tokenizer reads from the `chars` iterator and produce `Token`
pub struct Tokenizer<I: Iterator> {
    /// The chars iterators
    chars: IteratorWithPosition<I>,

    /// The current comment if any
    pub comment: Option<Comment>,
}

impl<I: Iterator<Item = char>> Tokenizer<I> {
    /// Returns a new Tokenizer for the given char iterator
    pub fn new(chars: I) -> Self {
        Self {
            chars: IteratorWithPosition::new(chars),
            comment: None,
        }
    }

    /// Returns the current line
    pub fn current_line(&self) -> usize {
        self.chars.current_line()
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

        for char in &mut self.chars {
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

        while let Some(char) = self
            .chars
            .next_if(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_'))
        {
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
    fn read_comment(&mut self) -> Result<Comment, TokenError> {
        let char = self.chars.next().ok_or(TokenError::EOF)?;
        let start_line = self.current_line();

        match char {
            // /* slash star comment */
            '*' => {
                let mut previous_char = self.chars.next().ok_or(TokenError::EOF)?;

                // ignore second * for block comments starting with /**
                if previous_char == '*' {
                    previous_char = self.chars.next().ok_or(TokenError::EOF)?;
                }

                let mut comment = String::new();
                let mut last_insert_is_line = false;

                while let Some(current_char) = self.chars.next() {
                    match (previous_char, current_char) {
                        // return comment when we get a */
                        ('*', '/') => {
                            return Ok(Comment::star_slash(
                                comment,
                                start_line,
                                self.current_line(),
                            ));
                        }

                        // skip \r
                        ('\r', _) => {
                            previous_char = current_char;
                        }

                        // skip whitespace after a new line
                        ('\n', ' ' | '\t') => {}

                        _ => {
                            match (last_insert_is_line, previous_char) {
                                (true, '*') => {}
                                (_, '\n') => {
                                    last_insert_is_line = true;
                                    comment.push(previous_char);
                                }
                                _ => comment.push(previous_char),
                            }

                            previous_char = current_char;
                        }
                    }
                }

                Ok(Comment::star_slash(
                    comment,
                    start_line,
                    self.current_line(),
                ))
            }

            // // double slash comment
            '/' => {
                let mut comment = String::new();
                let mut stripped_first_slash = false;
                while let Some(c) = self.chars.next_if(|c| *c != '\n') {
                    if stripped_first_slash {
                        comment.push(c);
                    } else {
                        stripped_first_slash = true;
                        if c != '/' {
                            comment.push(c);
                        }
                    }
                }

                Ok(match self.comment.take() {
                    // Concat with the previous double slash comment if it directly preceed this one
                    Some(Comment {
                        kind: CommentKind::DoubleSlash,
                        text,
                        start_line: previous_start_line,
                        end_line,
                        ..
                    }) if end_line == start_line - 1 => Comment::double_slash(
                        format!("{}\n{}", text, comment),
                        previous_start_line,
                        start_line,
                    ),
                    _ => Comment::double_slash(comment, start_line, start_line),
                })
            }

            found => Err(TokenError::UnexpectedChar(found)),
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

            // whitespace or New line
            Some(' ') | Some('\t') | Some('\r') | Some('\n') => self.next(),

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
    fn it_should_parse_double_quote_string() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new(r#""hello world""#.chars());
        assert_eq!(tokenizer.next()?, Token::String("hello world".to_string()));
        Ok(())
    }

    #[test]
    fn it_should_parse_double_slash_comment() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new("// hello world".chars());
        tokenizer.next()?;
        assert_eq!(
            tokenizer.comment.map(|c| c.text),
            Some(" hello world".into())
        );
        Ok(())
    }

    #[test]
    fn it_should_parse_triple_slash_comment() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new("/// hello world".chars());
        tokenizer.next()?;
        assert_eq!(
            tokenizer.comment.map(|c| c.text),
            Some(" hello world".into())
        );
        Ok(())
    }

    #[test]
    fn it_should_parse_multiline_double_slash_comment() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new("// hello\n// world".chars());
        tokenizer.next()?;
        assert_eq!(
            tokenizer.comment.map(|c| c.text),
            Some(" hello\n world".into())
        );
        Ok(())
    }

    #[test]
    fn it_should_parse_slash_star_comment() -> Result<(), TokenError> {
        let mut tokenizer = Tokenizer::new("/* hello world */".chars());
        tokenizer.next()?;

        assert_eq!(
            tokenizer.comment.map(|c| c.text),
            Some(" hello world ".into())
        );
        Ok(())
    }

    #[test]
    fn it_should_parse_doc_string() -> Result<(), TokenError> {
        let comment = r#"
        /**
         * Block comment l1
         * Block comment l2
         * Block comment l3
         */          
        "#;

        let mut tokenizer = Tokenizer::new(comment.chars());
        tokenizer.next()?;
        assert_eq!(
            tokenizer.comment.map(|c| c.text),
            Some("\n Block comment l1\n Block comment l2\n Block comment l3\n".into())
        );
        Ok(())
    }
}
