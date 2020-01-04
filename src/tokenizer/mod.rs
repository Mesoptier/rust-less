mod helpers;

use alloc::collections::VecDeque;
use std::str::CharIndices;
use crate::tokenizer::helpers::*;
use std::iter::Peekable;
use std::borrow::{Cow, Borrow};

// https://www.w3.org/TR/css-syntax-3/#tokenization
#[derive(Debug, PartialEq, Clone)]
pub enum Token<'i> {
    Ident(Cow<'i, str>),
    Function(Cow<'i, str>),
    AtKeyword(Cow<'i, str>),
    Hash {
        is_id: bool,
        value: Cow<'i, str>,
    },
    String {
        /// Value inside the quotes.
        value: Cow<'i, str>,
    },
    BadString,
    Url,
    BadUrl,
    Delim(char),
    Number {
        value: &'i str,
        is_integer: bool,
    },
    Percentage {
        value: &'i str,
    },
    Dimension {
        value: &'i str,
        is_integer: bool,
        unit: Cow<'i, str>,
    },
    Whitespace(&'i str),
    CDO,
    CDC,
    Colon,
    Semicolon,
    Comma,
    LeftSquareBracket,
    RightSquareBracket,
    LeftParenthesis,
    RightParenthesis,
    LeftCurlyBracket,
    RightCurlyBracket,
}

pub struct Tokenizer<'i> {
    input: &'i str,
    position: usize,
    iter: Peekable<CharIndices<'i>>,
}

impl<'i> Tokenizer<'i> {
    pub fn new(input: &'i str) -> Tokenizer {
        Self {
            input,
            position: 0,
            iter: input.char_indices().peekable(),
        }
    }

    fn advance(&mut self, dist: usize) {
        for i in 0..dist {
            match self.iter.next() {
                Some((p, _)) => {
                    self.position = p + 1;
                }
                _ => {}
            }
        }
    }

    fn consume(&mut self) -> Option<char> {
        match self.iter.next() {
            Some((p, c)) => {
                self.position = p + 1;
                Some(c)
            }
            None => None,
        }
    }

    fn lookahead(&mut self, dist: usize) -> Option<char> {
        if dist == 0 {
            return self.iter.peek().map(|(_, c)| *c);
        }
        self.iter.clone().nth(dist).map(|(_, c)| c)
    }

    fn lookahead_pair(&self) -> (Option<char>, Option<char>) {
        let mut iter = self.iter.clone();
        (
            iter.next().map(|(_, c)| c),
            iter.next().map(|(_, c)| c),
        )
    }

    fn lookahead_triple(&self) -> (Option<char>, Option<char>, Option<char>) {
        let mut iter = self.iter.clone();
        (
            iter.next().map(|(_, c)| c),
            iter.next().map(|(_, c)| c),
            iter.next().map(|(_, c)| c),
        )
    }

    /// Consume the next token.
    fn consume_token(&mut self) -> Option<Token<'i>> {
        // TODO: Use utf-8/utf8 crate to get the next code-point
        // TODO: Preprocess input stream (https://www.w3.org/TR/css-syntax-3/#input-preprocessing)
        // TODO: self.consume_comments();

        match self.lookahead(0) {
            Some(c) => Some(match c {
                c if is_whitespace(c) => self.consume_whitespace(),
                '"' => self.consume_string('"'),
                '#' => self.consume_number_sign(),
                '\'' => self.consume_string('\''),
                '(' => {
                    self.advance(1);
                    Token::LeftParenthesis
                }
                ')' => {
                    self.advance(1);
                    Token::RightParenthesis
                }
                '+' => self.consume_plus_sign(),
                ',' => {
                    self.advance(1);
                    Token::Comma
                }
                '-' => self.consume_minus_sign(),
                '.' => self.consume_full_stop(),
                ':' => {
                    self.advance(1);
                    Token::Colon
                }
                ';' => {
                    self.advance(1);
                    Token::Semicolon
                }
                '<' => {
                    self.advance(1);
                    match self.lookahead_triple() {
                        (Some('!'), Some('-'), Some('-')) => Token::CDO,
                        _ => Token::Delim('<'),
                    }
                }
                '@' => {
                    self.advance(1);
                    match self.lookahead_triple() {
                        (Some(c1), Some(c2), Some(c3)) if would_start_identifier(c1, c2, c3) =>
                            Token::AtKeyword(self.consume_name()),
                        _ => Token::Delim('@'),
                    }
                }
                '[' => {
                    self.advance(1);
                    Token::LeftSquareBracket
                }
                '\\' => match self.lookahead_pair() {
                    (Some(c1), Some(c2)) if is_valid_escape(c1, c2) => {
                        self.consume_identlike()
                    }
                    _ => {
                        // parse error
                        self.advance(1);
                        Token::Delim('\\')
                    }
                }
                ']' => {
                    self.advance(1);
                    Token::RightSquareBracket
                }
                '{' => {
                    self.advance(1);
                    Token::LeftCurlyBracket
                }
                '}' => {
                    self.advance(1);
                    Token::RightCurlyBracket
                }
                c if is_digit(c) => {
                    self.consume_numeric()
                }
                c if is_name_start(c) => {
                    self.consume_identlike()
                }
                c => {
                    self.advance(1);
                    Token::Delim(c)
                }
            }),
            None => None,
        }
    }

    /// Consume a whitespace token.
    ///
    /// Precondition: next char is whitespace.
    fn consume_whitespace(&mut self) -> Token<'i> {
        let start = self.position;
        self.advance(1);
        loop {
            match self.lookahead(0) {
                Some(c) if is_whitespace(c) => {
                    self.consume();
                }
                _ => {
                    break;
                }
            }
        }
        Token::Whitespace(&self.input[start..=self.position - 1])
    }

    /// Consume a token starting with '#'.
    ///
    /// Precondition: next char is '#'.
    fn consume_number_sign(&mut self) -> Token<'i> {
        self.advance(1);
        match self.lookahead_triple() {
            (Some(c1), Some(c2), Some(c3)) if is_name(c1) || is_valid_escape(c1, c2) => {
                let is_id = would_start_identifier(c1, c2, c3);
                let value = self.consume_name();
                Token::Hash { is_id, value }
            }
            _ => {
                Token::Delim('#')
            }
        }
    }

    /// Consume a token starting with '+'.
    ///
    /// Precondition: next char is '+'.
    fn consume_plus_sign(&mut self) -> Token<'i> {
        match self.lookahead_triple() {
            (Some(c1), Some(c2), Some(c3)) if would_start_number(c1, c2, c3) => {
                self.consume_numeric()
            }
            _ => {
                self.advance(1);
                Token::Delim('+')
            }
        }
    }

    /// Consume a token starting with '-'.
    ///
    /// Precondition: next char is '-'.
    fn consume_minus_sign(&mut self) -> Token<'i> {
        match self.lookahead_triple() {
            (Some(c1), Some(c2), Some(c3)) if would_start_number(c1, c2, c3) => {
                self.consume_numeric()
            }
            (_, Some('-'), Some('>')) => {
                self.advance(3);
                Token::CDC
            }
            (Some(c1), Some(c2), Some(c3)) if would_start_identifier(c1, c2, c3) => {
                self.consume_identlike()
            }
            _ => {
                self.advance(1);
                Token::Delim('-')
            }
        }
    }

    fn consume_full_stop(&mut self) -> Token<'i> {
        match self.lookahead_triple() {
            (Some(c1), Some(c2), Some(c3)) if would_start_number(c1, c2, c3) => {
                self.consume_numeric()
            }
            _ => {
                self.advance(1);
                Token::Delim('.')
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-numeric-token
    fn consume_numeric(&mut self) -> Token<'i> {
        let (value, is_integer) = self.consume_number();

        match self.lookahead_triple() {
            (Some(c1), Some(c2), Some(c3)) if would_start_identifier(c1, c2, c3) =>
                Token::Dimension { value, is_integer, unit: self.consume_name() },
            (Some(c1 @ '%'), _, _) => {
                self.advance(1);
                Token::Percentage { value }
            }
            _ => Token::Number { value, is_integer }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-number
    fn consume_number(&mut self) -> (&'i str, bool) {
        let start_pos = self.position;
        let mut is_integer = true;

        match self.lookahead(0) {
            Some(c @ '+') | Some(c @ '-') => {
                self.advance(1);
            }
            _ => {}
        }

        self.consume_digits();

        match self.lookahead_pair() {
            (Some(c1 @ '.'), Some(c2)) if is_digit(c2) => {
                self.advance(2);
                is_integer = false;
                self.consume_digits();
            }
            _ => {}
        }

        match self.lookahead(0) {
            Some(c1 @ 'e') | Some(c1 @ 'E') => match self.lookahead(1) {
                Some(c2) if is_digit(c2) => {
                    self.advance(1);
                    is_integer = false;
                    self.consume_digits();
                }
                Some(c2 @ '+') | Some(c2 @ '-') => match self.lookahead(2) {
                    Some(c3) if is_digit(c3) => {
                        self.advance(3);
                        is_integer = false;
                        self.consume_digits();
                    }
                    _ => {}
                }
                _ => {}
            }
            _ => {}
        }

        // TODO: Convert repr to number (f32) while constructing repr?

        (&self.input[start_pos..self.position], is_integer)
    }

    fn consume_digits(&mut self) {
        loop {
            match self.lookahead(0) {
                Some(c) if is_digit(c) => {
                    self.advance(1);
                }
                _ => break
            }
        }
    }

    /// Consume an ident-like token.
    fn consume_identlike(&mut self) -> Token<'i> {
        let value = self.consume_name();

        if self.lookahead(0) == Some('(') {
            if value.eq_ignore_ascii_case("url") {
                self.advance(1);

                while let (Some(c1), Some(c2)) = self.lookahead_pair() {
                    if is_whitespace(c1) && is_whitespace(c2) {
                        self.advance(1);
                    } else {
                        break;
                    }
                }

                return match self.lookahead_pair() {
                    (Some('"'), _) | (Some('\''), _) => {
                        Token::Function(value)
                    }
                    (Some(c), Some('"')) | (Some(c), Some('\'')) if is_whitespace(c) => {
                        Token::Function(value)
                    }
                    _ => {
                        self.consume_url()
                    }
                };
            } else {
                return Token::Function(value);
            }
        }

        Token::Ident(value)
    }

    /// Consume a string token.
    ///
    /// Precondition: next char is `"` or `'` and equals `ending`.
    fn consume_string(&mut self, ending: char) -> Token<'i> {
        self.advance(1);
        match self.consume_string_tail(ending) {
            Some(value) => Token::String { value },
            None => Token::BadString,
        }
    }

    fn consume_string_tail(&mut self, ending: char) -> Option<Cow<'i, str>> {
        let start_pos = self.position;
        let mut value_chars;

        loop {
            match self.lookahead(0) {
                Some(c) if c == ending => {
                    self.advance(1);
                    return Some(self.input[start_pos..self.position].borrow().into());
                }
                None => {
                    // parse error
                    return Some(self.input[start_pos..self.position].borrow().into());
                }
                Some('\n') => {
                    // parse error
                    return None;
                }
                Some('\\') => {
                    value_chars = self.input[start_pos..self.position].to_owned();
                    break;
                }
                Some(_) => {
                    self.advance(1);
                }
            }
        }

        // TODO: Parse escaped codepoints
        Some(Cow::from("<ESCAPE CODEPOINTS>"))
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-url-token
    fn consume_url(&mut self) -> Token<'i> {
        // TODO: consume URL token
        Token::Url
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-name
    fn consume_name(&mut self) -> Cow<'i, str> {
        let start_pos = self.position;
        let mut value_chars;

        loop {
            match self.lookahead(0) {
                Some(c) if is_name(c) => {
                    self.advance(1);
                }
                Some('\\') => {
                    value_chars = self.input[start_pos..self.position].to_owned();
                    break;
                }
                _ => {
                    return self.input[start_pos..self.position].borrow().into();
                }
            }
        }

        // TODO: Parse escaped codepoints
        Cow::from("<ESCAPE CODEPOINTS>")
    }
}

impl<'i> Iterator for Tokenizer<'i> {
    type Item = Token<'i>;

    fn next(&mut self) -> Option<Token<'i>> {
        self.consume_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cases() {
        let cases: Vec<(&str, Vec<Token>)> = vec![
            ("ident", vec![Token::Ident("ident".into())]),
            ("func()", vec![Token::Function("func".into()), Token::LeftParenthesis, Token::RightParenthesis]),
            ("@at-keyword", vec![Token::AtKeyword("at-keyword".into())]),
            ("{", vec![Token::LeftCurlyBracket]),
            ("}", vec![Token::RightCurlyBracket]),
            ("(", vec![Token::LeftParenthesis]),
            (")", vec![Token::RightParenthesis]),
            ("[", vec![Token::LeftSquareBracket]),
            ("]", vec![Token::RightSquareBracket]),
        ];

        for (input, expected) in cases {
            let tokenizer = Tokenizer::new(input);
            assert_eq!(tokenizer.collect::<Vec<Token>>(), expected);
        }
    }

    #[test]
    fn test() {
        let input: &str = r#"
#lib() {
  .colors() {
    @primary: blue;
    @secondary: green;
  }
  .rules(@size) {
    border: @size solid white;
  }

  @test();
  @test[];
  @test[param];

  @test-asd: "blue";
}

.box when (#lib.colors[@primary] = blue) {
  width: 100px;
  height: ($width / 2);
}

.bar:extend(.box) {
  @media (min-width: 600px) {
    width: 200px;
    #lib.rules(1px);
  }
}
"#;
        let mut tokenizer = Tokenizer::new(input);
        for token in tokenizer {
            println!("{:?}", token);
        }
    }
}