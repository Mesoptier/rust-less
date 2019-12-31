mod helpers;

use alloc::collections::VecDeque;
use std::str::Chars;
use crate::tokenizer::helpers::*;
use crate::stream::{Stream, PeekTuple, PeekAt};

// https://www.w3.org/TR/css-syntax-3/#tokenization
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    EOF,
    Ident {
        value: String,
    },
    Function {
        value: String,
    },
    AtKeyword {
        value: String,
    },
    Hash {
        is_id: bool,
        value: String,
    },
    String {
        value: String,
    },
    BadString,
    Url,
    BadUrl,
    Delim {
        value: char,
    },
    Number {
        value: String,
        is_integer: bool,
    },
    Percentage {
        value: String,
    },
    Dimension {
        value: String,
        is_integer: bool,
        unit: String,
    },
    Whitespace,
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

type Codepoints<L: Iterator<Item=char>> = Stream<char, L, [Option<char>; 3]>;

pub struct Tokenizer<'i> {
    input: Codepoints<Chars<'i>>,
}

impl<'i> Tokenizer<'i> {
    pub fn new(input: Chars<'i>) -> Self {
        Self {
            input: Codepoints::new(input)
        }
    }

    /// 4.3.1. Consume a token
    fn consume_token(&mut self) -> Token {
        // TODO: Use utf-8/utf8 crate to get the next code-point
        // TODO: Preprocess input stream (https://www.w3.org/TR/css-syntax-3/#input-preprocessing)
        // TODO: self.consume_comments();

        match self.input.consume() {
            Some(codepoint) => match codepoint {
                c if is_whitespace(c) => self.consume_whitespace(),
                '"' => self.consume_string('"'),
                '#' => self.consume_number_sign(),
                '\'' => self.consume_string('\''),
                '(' => Token::LeftParenthesis,
                ')' => Token::RightParenthesis,
                '+' => self.consume_plus_sign(),
                ',' => Token::Comma,
                '-' => self.consume_minus_sign(),
                '.' => self.consume_full_stop(),
                ':' => Token::Colon,
                ';' => Token::Semicolon,
                '<' => match self.input.peek_tuple(0) {
                    (Some('!'), Some('-'), Some('-')) => Token::CDO,
                    _ => Token::Delim { value: '<' },
                },
                '@' => match self.input.peek_tuple(0) {
                    (Some(c1), Some(c2), Some(c3)) if would_start_identifier(c1, c2, c3) =>
                        Token::AtKeyword { value: self.consume_name() },
                    _ => Token::Delim { value: '@' },
                }
                '[' => Token::LeftSquareBracket,
                '\\' => match self.input.peek_tuple(-1) {
                    (Some(c1), Some(c2)) if is_valid_escape(c1, c2) => {
                        self.input.reconsume_current();
                        self.consume_identlike()
                    }
                    _ => /* parse error */ Token::Delim { value: '\\' },
                }
                ']' => Token::RightSquareBracket,
                '{' => Token::LeftCurlyBracket,
                '}' => Token::RightCurlyBracket,
                c if is_digit(c) => {
                    self.input.reconsume_current();
                    self.consume_numeric()
                }
                c if is_name_start(c) => {
                    self.input.reconsume_current();
                    self.consume_identlike()
                }
                c => Token::Delim { value: c },
            },
            None => Token::EOF,
        }
    }

    fn consume_whitespace(&mut self) -> Token {
        loop {
            match self.input.consume() {
                Some(c) if is_whitespace(c) => {}
                _ => {
                    self.input.reconsume_current();
                    break;
                }
            }
        }

        Token::Whitespace
    }

    fn consume_number_sign(&mut self) -> Token {
        match (self.input.peek_at(0), self.input.peek_at(1), self.input.peek_at(2)) {
            (Some(c1), Some(c2), Some(c3)) if is_name(c1) || is_valid_escape(c1, c2) => {
                let is_id = would_start_identifier(c1, c2, c3);
                let value = self.consume_name();
                Token::Hash { is_id, value }
            }
            _ => Token::Delim { value: '#' },
        }
    }

    fn consume_plus_sign(&mut self) -> Token {
        match self.input.peek_tuple(-1) {
            (Some(c1), Some(c2), Some(c3)) if would_start_number(c1, c2, c3) => {
                self.input.reconsume_current();
                self.consume_numeric()
            }
            _ => Token::Delim { value: '+' },
        }
    }

    fn consume_minus_sign(&mut self) -> Token {
        match self.input.peek_tuple(-1) {
            (Some(c1), Some(c2), Some(c3)) if would_start_number(c1, c2, c3) => {
                self.input.reconsume_current();
                self.consume_numeric()
            }
            (_, Some('-'), Some('>')) => {
                self.input.consume();
                self.input.consume();
                Token::CDC
            }
            (Some(c1), Some(c2), Some(c3)) if would_start_identifier(c1, c2, c3) => {
                self.input.reconsume_current();
                self.consume_identlike()
            }
            _ => Token::Delim { value: '-' },
        }
    }

    fn consume_full_stop(&mut self) -> Token {
        match self.input.peek_tuple(-1) {
            (Some(c1), Some(c2), Some(c3)) if would_start_number(c1, c2, c3) => {
                self.input.reconsume_current();
                self.consume_numeric()
            }
            _ => Token::Delim { value: '.' },
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-numeric-token
    fn consume_numeric(&mut self) -> Token {
        let (value, is_integer) = self.consume_number();

        match self.input.peek_tuple(0) {
            (Some(c1), Some(c2), Some(c3)) if would_start_identifier(c1, c2, c3) =>
                Token::Dimension { value, is_integer, unit: self.consume_name() },
            (Some(c1 @ '%'), _, _) => {
                self.input.consume();
                Token::Percentage { value }
            }
            _ => Token::Number { value, is_integer }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-number
    fn consume_number(&mut self) -> (String, bool) {
        let mut repr = String::from("");
        let mut is_integer = true;

        match self.input.peek_at(0) {
            Some(c @ '+') | Some(c @ '-') => {
                self.input.consume();
                repr.push(c);
            }
            _ => {}
        }

        repr.push_str(self.consume_digits().as_str());

        match self.input.peek_tuple(0) {
            (Some(c1 @ '.'), Some(c2)) if is_digit(c2) => {
                self.input.consume();
                self.input.consume();
                repr.push(c1);
                repr.push(c2);
                is_integer = false;
                repr.push_str(self.consume_digits().as_str());
            }
            _ => {}
        }

        match self.input.peek_at(0) {
            Some(c1 @ 'e') | Some(c1 @ 'E') => match self.input.peek_at(1) {
                Some(c2) if is_digit(c2) => {
                    self.input.consume();
                    self.input.consume();
                    repr.push(c1);
                    repr.push(c2);
                    is_integer = false;
                    repr.push_str(self.consume_digits().as_str());
                }
                Some(c2 @ '+') | Some(c2 @ '-') => match self.input.peek_at(2) {
                    Some(c3) if is_digit(c3) => {
                        self.input.consume();
                        self.input.consume();
                        self.input.consume();
                        repr.push(c1);
                        repr.push(c2);
                        repr.push(c3);
                        is_integer = false;
                        repr.push_str(self.consume_digits().as_str());
                    }
                    _ => {}
                }
                _ => {}
            }
            _ => {}
        }

        // TODO: Convert repr to number (f32) while constructing repr?

        (repr, is_integer)
    }

    fn consume_digits(&mut self) -> String {
        let mut repr = String::new();
        loop {
            match self.input.consume() {
                Some(c) if is_digit(c) => {
                    repr.push(c);
                }
                _ => {
                    self.input.reconsume_current();
                    return repr;
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-ident-like-token
    fn consume_identlike(&mut self) -> Token {
        let value = self.consume_name();

        if self.input.peek_at(0) == Some('(') {
            if value.eq_ignore_ascii_case("url") {
                self.input.consume();

                while let (Some(c1), Some(c2)) = self.input.peek_tuple(0) {
                    if is_whitespace(c1) && is_whitespace(c2) {
                        self.input.consume();
                    } else {
                        break;
                    }
                }

                match self.input.peek_tuple(0) {
                    (Some('"'), _) | (Some('\''), _) => {
                        return Token::Function { value };
                    }
                    (Some(c), Some('"')) | (Some(c), Some('\'')) if is_whitespace(c) => {
                        return Token::Function { value };
                    }
                    _ => {
                        return self.consume_url();
                    }
                }
            } else {
                return Token::Function { value };
            }
        }

        Token::Ident { value }
    }

    /// 4.3.5. Consume a string token
    fn consume_string(&mut self, ending: char) -> Token {
        let mut value = String::from("");

        loop {
            match self.input.consume() {
                Some(c) if c == ending => {
                    return Token::String { value };
                }
                None => /* parse error */ {
                    return Token::String { value };
                }
                Some('\n') => /* parse error */ {
                    self.input.reconsume_current();
                    return Token::BadString;
                }
                Some('\\') => {
                    match self.input.peek_at(0) {
                        None => {}
                        Some('\n') => {
                            self.input.consume();
                        }
                        Some(_) => {
                            value.push(self.consume_escaped_codepoint());
                        }
                    }
                }
                Some(c) => {
                    value.push(c);
                }
            }
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-url-token
    fn consume_url(&mut self) -> Token {
        // TODO: consume URL token

        Token::Url
    }

    /// 4.3.7. Consume an escaped code point
    fn consume_escaped_codepoint(&mut self) -> char {
        match self.input.consume() {
            Some(c) if is_hex_digit(c) => {
                let mut digits = String::from("");
                digits.push(c);

                // Consume another 1-5 hex digits
                for i in 1..=5 {
                    match self.input.consume() {
                        Some(c) if is_hex_digit(c) => digits.push(c),
                        _ => {
                            self.input.reconsume_current();
                            break;
                        }
                    }
                }

                // If the next codepoint is whitespace, consume it as well
                if let Some(c) = self.input.peek_at(0) {
                    if is_whitespace(c) {
                        self.input.consume();
                    }
                }

                u32::from_str_radix(&digits, 16)
                    .map(|i| core::char::from_u32(i).unwrap_or(core::char::REPLACEMENT_CHARACTER))
                    .unwrap_or(core::char::REPLACEMENT_CHARACTER)
            }
            None => /* parse error */ core::char::REPLACEMENT_CHARACTER,
            Some(c) => c,
        }
    }

    /// https://www.w3.org/TR/css-syntax-3/#consume-name
    fn consume_name(&mut self) -> String {
        let mut result = String::from("");
        loop {
            if let Some(c1) = self.input.consume() {
                if is_name(c1) {
                    result.push(c1);
                    continue;
                }
                if let Some(c2) = self.input.peek_at(0) {
                    if is_valid_escape(c1, c2) {
                        result.push(self.consume_escaped_codepoint());
                        continue;
                    }
                }
            }

            self.input.reconsume_current();
            return result;
        }
    }
}

impl<'i> Iterator for Tokenizer<'i> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        match self.consume_token() {
            Token::EOF => None,
            token => Some(token),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut tokenizer = Tokenizer::new(input.chars());
        for token in tokenizer {
            println!("{:?}", token);
        }
    }
}