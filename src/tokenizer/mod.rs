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
    Hash,
    String(String),
    BadString,
    Url,
    BadUrl,
    Delim(char),
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
            buffer: Default::default()
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
        Self{
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
                '\n' | '\t' | ' ' => self.consume_whitespace(),
                '"' => self.consume_string('"'),
//                '#' => {},
                '\'' => self.consume_string('\''),
                c => Token::Delim(c),
            },
            None => Token::EOF,
        }
    }

    fn consume_whitespace(&mut self) -> Token {
        loop {
            match self.input.consume() {
                Some('\n') | Some('\t') | Some(' ') => {}
                codepoint => {
                    self.input.reconsume(codepoint);
                    break;
                }
            }
        }

        Token::Whitespace
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
                        },
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