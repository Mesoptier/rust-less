mod helpers;

use alloc::collections::VecDeque;
use std::str::Chars;
use crate::tokenizer::helpers::*;

// https://www.w3.org/TR/css-syntax-3/#tokenization
#[derive(Debug, PartialEq)]
enum Token {
    EOF,
    Ident,
    Function,
    AtKeyword,
    Hash(
        bool, // type_id flag
        String, // value
    ),
    String(
        String, // value
    ),
    BadString,
    Url,
    BadUrl,
    Delim(
        char, // value
    ),
    Number,
    Percentage,
    Dimension,
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

struct Codepoints<L> {
    input: L,
    current: Option<char>,

    // TODO: Use arraydeque, since CSS tokenizer only does ~3 lookahead + ~1 reconsume
    buffer: VecDeque<Option<char>>,
}

impl<L> Codepoints<L>
    where L: Iterator<Item=char>,
{
    pub fn new(input: L) -> Self {
        Self {
            input,
            current: None,
            buffer: Default::default(),
        }
    }

    fn consume(&mut self) -> Option<char> {
        self.current = match self.buffer.pop_front() {
            Some(value) => value,
            None => self.input.next(),
        };
        self.current
    }

    fn reconsume(&mut self, value: Option<char>) {
        self.buffer.push_front(value);
    }

    fn peek(&mut self, index: usize) -> Option<char> {
        // Fill buffer until it contains index
        while index + 1 > self.buffer.len() {
            self.buffer.push_back(self.input.next());
        }
        *self.buffer.get(index).unwrap()
    }
}

struct Tokenizer<'i> {
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
                c => Token::Delim(c),
            },
            None => Token::EOF,
        }
    }

    fn consume_whitespace(&mut self) -> Token {
        loop {
            match self.input.consume() {
                Some(c) if is_whitespace(c) => {}
                codepoint => {
                    self.input.reconsume(codepoint);
                    break;
                }
            }
        }

        Token::Whitespace
    }

    fn consume_number_sign(&mut self) -> Token {
        match (self.input.peek(0), self.input.peek(1), self.input.peek(2)) {
            (Some(c1), Some(c2), Some(c3)) if is_name(c1) || is_valid_escape(c1, c2) => {
                let type_id = would_start_identifier(c1, c2, c3);
                let value = self.consume_name();
                Token::Hash(type_id, value)
            }
            _ => Token::Delim('#'),
        }
    }

    /// 4.3.5. Consume a string token
    fn consume_string(&mut self, ending: char) -> Token {
        let mut value = String::from("");

        loop {
            match self.input.consume() {
                Some(c) if c == ending => {
                    return Token::String(value);
                }
                None => /* parse error */ {
                    return Token::String(value);
                }
                Some('\n') => /* parse error */ {
                    self.input.reconsume(self.input.current);
                    return Token::BadString;
                }
                Some('\\') => {
                    match self.input.peek(0) {
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
                        value => {
                            self.input.reconsume(value);
                            break;
                        }
                    }
                }

                // If the next codepoint is whitespace, consume it as well
                if let Some(c) = self.input.peek(0) {
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
                if let Some(c2) = self.input.peek(0) {
                    if is_valid_escape(c1, c2) {
                        result.push(self.consume_escaped_codepoint());
                        continue;
                    }
                }
            }

            self.input.reconsume(self.input.current);
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
        let input: &str = "test 'string'";
        let mut tokenizer = Tokenizer::new(input.chars());
        for token in tokenizer {
            println!("{:?}", token);
        }
    }
}