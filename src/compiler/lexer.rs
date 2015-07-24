use compiler::{Symbol, Token};
use std::ascii::AsciiExt;
use std::io;
use std::io::{BufRead};
use util::PeekableBuffer;

fn is_operator(c: char) -> bool {
    match c {
        '~' | '&' | '|' | '*' | '/' | '\\' | '+' | '=' | '>' | '<' | ',' | '@' | '%' => true,
        _ => false,
    }
}

fn is_identifier(c: char) -> bool {
    c.is_ascii() && (c.is_alphanumeric() || c == '_')
}

#[derive(Debug)]
enum Error {
    IoError(io::Error),
    End,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

pub struct Lexer<R: BufRead> {
    buffer: PeekableBuffer<R>,
    token_queue: Vec<Token>,
}

impl<R: BufRead> Iterator for Lexer<R> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        match self.read_token() {
            Ok(t) => Some(t),
            Err(_) => None
        }
    }
}

impl<R: BufRead> Lexer<R> {
    pub fn new(reader: R) -> Lexer<R> {
        Lexer {
            buffer: PeekableBuffer::new(reader),
            token_queue: Vec::new(),
        }
    }

    fn read_token(&mut self) -> Result<Token, Error> {
        if !self.token_queue.is_empty() {
            return Ok(self.token_queue.pop().unwrap());
        }

        loop {
            self.skip_whitespace();
            self.skip_comments();

            if self.buffer.peek().map_or(false, |c| c.is_whitespace()) {
                continue;
            } else {
                break;
            }
        }

        let c = match self.buffer.peek() {
            Some(c) => c,
            None => return Err(Error::End),
        };

        let token = match c {
            '[' => self.read_symbol(Symbol::NewBlock),
            ']' => self.read_symbol(Symbol::EndBlock),
            '(' => self.read_symbol(Symbol::NewTerm),
            ')' => self.read_symbol(Symbol::EndTerm),
            '#' => self.read_symbol(Symbol::Pound),
            '^' => self.read_symbol(Symbol::Exit),
            '.' => self.read_symbol(Symbol::Period),
            '-' => self.read_minus(),
            ':' => self.read_colon(),
            'a'...'z' | 'A'...'Z' => self.read_identifier(),
            '0'...'9' => self.read_number(),
            '\'' => self.read_string(),
            c if is_operator(c) => self.read_operator(),
            c  => panic!("do not understand: {:?}", c)
        };

        Ok(token)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.buffer.peek() {
                Some(c) if c.is_whitespace() => self.buffer.consume(),
                _ => break
            }
        }
    }

    fn skip_comments(&mut self) {
        if self.buffer.peek() != Some('"') {
            return;
        }

        self.buffer.consume();
        loop {
            if self.buffer.next() == Some('"') {
                break;
            }
        }
    }

    fn read_symbol(&mut self, symbol: Symbol) -> Token {
        self.buffer.consume();
        From::from(symbol)
    }

    fn read_operator(&mut self) -> Token {
        let c = self.buffer.next().unwrap();
        match c {
           '~' => From::from(Symbol::Not),
           '&' => From::from(Symbol::And),
           '|' => From::from(Symbol::Or),
           '*' => From::from(Symbol::Star),
           '/' => From::from(Symbol::Divide),
           '\\' => From::from(Symbol::Modulus),
           '+' => From::from(Symbol::Plus),
           '=' => From::from(Symbol::Equal),
           '>' => From::from(Symbol::More),
           '<' => From::from(Symbol::Less),
           ',' => From::from(Symbol::Comma),
           '@' => From::from(Symbol::At),
           '%' => From::from(Symbol::Percent),
           _ => unreachable!(),
       }
    }

    fn read_colon(&mut self) -> Token {
        self.buffer.consume();
        if self.buffer.peek() == Some('=') {
            self.buffer.consume();
            From::from(Symbol::Assign)
        } else {
            From::from(Symbol::Colon)
        }
    }

    fn read_identifier(&mut self) -> Token {
        let mut text = String::new();
        loop {
            match self.buffer.peek() {
                Some(c) if is_identifier(c) => {
                    text.push(c);
                    self.buffer.consume();
                }
                _ => break,
            }
        }

        if self.buffer.peek() == Some(':') {
            self.buffer.consume();
            text.push(':');

            let saw_sequence = self.buffer.peek().and_then(|c| {
                Some(c.is_alphabetic() && c.is_ascii())
            }).unwrap_or(false);
            if saw_sequence {
                loop {
                    match self.buffer.peek() {
                        Some(c @ 'a'...'z') | Some(c @ 'A'...'Z') | Some(c @ '0'...'9') | Some(c @ ':') => {
                            text.push(c);
                            self.buffer.consume();
                        }
                        _ => break,
                    }
                }

                Token(Symbol::KeywordSequence, Some(text))
            } else {
                Token(Symbol::Keyword, Some(text))
            }
        } else if text == "primitive" {
            From::from(Symbol::Primitive)
        } else {
            Token(Symbol::Identifier, Some(text))
        }
    }

    fn read_string(&mut self) -> Token {
        let mut text = String::new();

        self.buffer.consume();
        loop {
            match self.buffer.next() {
                Some('\'') => break,
                Some(c) => text.push(c),
                None => break
            }
        }

        Token(Symbol::String, Some(text))
    }

    fn read_number(&mut self) -> Token {
        let mut text = String::new();

        loop {
            match self.buffer.peek() {
                Some(c @ '0'...'9') => {
                    text.push(c);
                    self.buffer.consume();
                }
                _ => break,
            }
        }

        let saw_decimal = self.buffer.peek().map_or(false, |c| c == '.');
        if saw_decimal {
            self.buffer.consume();
            let saw_digit = self.buffer.peek().map_or(false, |c| c.is_digit(10));
            if saw_digit {
                text.push('.');

                loop {
                    match self.buffer.peek() {
                        Some(c @ '0'...'9') => {
                            text.push(c);
                            self.buffer.consume();
                        }
                        _ => break,
                    }
                }

                Token(Symbol::Double, Some(text))
            } else {
                self.token_queue.push(From::from(Symbol::Period));
                Token(Symbol::Integer, Some(text))
            }
        } else {
            Token(Symbol::Integer, Some(text))
        }
    }

    fn read_minus(&mut self) -> Token {
        self.buffer.consume();
        let mut count = 1;

        loop {
            if self.buffer.peek() == Some('-') {
                self.buffer.consume();
                count += 1;

                if count == 4 {
                    return From::from(Symbol::Separator)
                }
            } else {
                break;
            }
        }

        count -= 1;
        for _ in (0..count) {
            self.token_queue.push(From::from(Symbol::Minus))
        }

        From::from(Symbol::Minus)
    }
}

#[cfg(test)]
mod test {
    use super::Lexer;
    use compiler::{Symbol, Token};

    #[test]
    fn test_skipping_whitespace() {
        let source = "\n Hello \n Test".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Identifier, Some("Hello".to_string())));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Identifier, Some("Test".to_string())));
    }

    #[test]
    fn test_skipping_comments() {
        let source = "\"Test\" Hello \"123\" Test".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Identifier, Some("Hello".to_string())));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Identifier, Some("Test".to_string())));
    }

    #[test]
    fn test_identifier() {
        let source = "Hello".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Identifier, Some("Hello".to_string())));
    }

    #[test]
    fn test_keyword() {
        let source = "foo:".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Keyword, Some("foo:".to_string())));
    }

    #[test]
    fn test_two_keyword_sequence() {
        let source = "foo:bar:".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::KeywordSequence, Some("foo:bar:".to_string())));
    }

    #[test]
    fn test_three_keyword_sequence() {
        let source = "foo:bar:baz:".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::KeywordSequence, Some("foo:bar:baz:".to_string())));
    }

    #[test]
    fn test_primitive() {
        let source = "primitive".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Primitive, None));
    }

    #[test]
    fn test_minus() {
        let source = "-".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Minus, None));
    }

    #[test]
    fn test_two_minus() {
        let source = "--".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Minus, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Minus, None));
    }

    #[test]
    fn test_three_minus() {
        let source = "---".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Minus, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Minus, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Minus, None));
    }

    #[test]
    fn test_separator() {
        let source = "-----".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Separator, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Minus, None));
    }

    #[test]
    fn test_integer() {
        let source = "1".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Integer, Some("1".to_string())));
    }

    #[test]
    fn test_integer_and_period() {
        let source = "1.".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Integer, Some("1".to_string())));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Period, None));
    }

    #[test]
    fn test_double() {
        let source = "3.14".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Double, Some("3.14".to_string())));
    }

    #[test]
    fn test_colon() {
        let source = ":".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Colon, None));
    }

    #[test]
    fn test_assignment() {
        let source = "foo := 'Hello'".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Identifier, Some("foo".to_string())));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Assign, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::String, Some("Hello".to_string())));
    }

    #[test]
    fn test_simple_symbols() {
        let source = "[]()#^.".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::NewBlock, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::EndBlock, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::NewTerm, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::EndTerm, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Pound, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Exit, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Period, None));
    }

    #[test]
    fn test_simple_operators() {
        let source = "~&|*/\\+=<>,@%".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Not, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::And, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Or, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Star, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Divide, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Modulus, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Plus, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Equal, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Less, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::More, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Comma, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::At, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Percent, None));
    }

    #[test]
    fn test_lexer() {
        let source = "
        Hello = (
            \"The 'run' method is called when initializing the system\"
            run = ('Hello, World from SOM' println)
        )
        ".as_bytes();
        let mut lexer = Lexer::new(source);
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Identifier, Some("Hello".to_string())));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Equal, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::NewTerm, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Identifier, Some(("run".to_string()))));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Equal, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::NewTerm, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::String, Some("Hello, World from SOM".to_string())));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::Identifier, Some("println".to_string())));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::EndTerm, None));
        assert_eq!(lexer.read_token().unwrap(), Token(Symbol::EndTerm, None));
    }
}
