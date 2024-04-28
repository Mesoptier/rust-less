use std::borrow::Cow;

use winnow::ascii::Caseless;
use winnow::combinator::{alt, delimited, empty, fail, opt, preceded, repeat, terminated};
use winnow::token::{any, one_of, take_until, take_while};
use winnow::{seq, Located, PResult, Parser};

use crate::lexer::helpers::{is_digit, is_name, would_start_identifier};

#[derive(Clone, Debug, PartialEq)]
pub enum Delim {
    Paren,
    Brace,
    Bracket,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'i> {
    Whitespace,
    Comment(Cow<'i, str>),
    Ident(Cow<'i, str>),
    Hash(Cow<'i, str>),
    String(Cow<'i, str>),
    Number(f32),
    OpenDelim(Delim),
    CloseDelim(Delim),
    Symbol(char),
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    tokenize_impl
        .parse(Located::new(input))
        .map_err(|e| e.to_string())
}

fn tokenize_impl<'i>(input: &mut Located<&'i str>) -> PResult<Vec<Token<'i>>> {
    repeat(0.., token).parse_next(input)
}

fn token<'i>(input: &mut Located<&'i str>) -> PResult<Token<'i>> {
    alt((
        whitespace,
        comment,
        ident,
        hash,
        string,
        number,
        '('.value(Token::OpenDelim(Delim::Paren)),
        '{'.value(Token::OpenDelim(Delim::Brace)),
        '['.value(Token::OpenDelim(Delim::Bracket)),
        ')'.value(Token::CloseDelim(Delim::Paren)),
        '}'.value(Token::CloseDelim(Delim::Brace)),
        ']'.value(Token::CloseDelim(Delim::Bracket)),
        any.map(Token::Symbol),
    ))
    .parse_next(input)
}

fn whitespace<'i>(input: &mut Located<&'i str>) -> PResult<Token<'i>> {
    take_while(1.., char::is_whitespace)
        .value(Token::Whitespace)
        .parse_next(input)
}

fn comment<'i>(input: &mut Located<&'i str>) -> PResult<Token<'i>> {
    alt((line_comment, block_comment)).parse_next(input)
}

fn line_comment<'i>(input: &mut Located<&'i str>) -> PResult<Token<'i>> {
    preceded("//", take_until(0.., "\n"))
        .map(|value: &str| Token::Comment(value.into()))
        .parse_next(input)
}

fn block_comment<'i>(input: &mut Located<&'i str>) -> PResult<Token<'i>> {
    delimited("/*", take_until(0.., "*/"), "*/")
        .map(|value: &str| Token::Comment(value.into()))
        .parse_next(input)
}

fn ident<'i>(input: &mut Located<&'i str>) -> PResult<Token<'i>> {
    preceded(peek_ident_start, ident_sequence)
        .map(|value| Token::Ident(value.into()))
        .parse_next(input)
}

fn ident_sequence<'i>(input: &mut Located<&'i str>) -> PResult<&'i str> {
    take_while(1.., is_name).parse_next(input)
}

/// Matches if the next characters would start an identifier.
fn peek_ident_start<'i>(input: &mut Located<&'i str>) -> PResult<()> {
    if would_start_identifier(input.as_ref()) {
        empty.parse_next(input)
    } else {
        fail.parse_next(input)
    }
}

fn hash<'i>(input: &mut Located<&'i str>) -> PResult<Token<'i>> {
    preceded('#', ident_sequence)
        .map(|value: &str| Token::Hash(value.into()))
        .parse_next(input)
}

fn string<'i>(input: &mut Located<&'i str>) -> PResult<Token<'i>> {
    let quote = one_of(|c| c == '"' || c == '\'').parse_next(input)?;
    // TODO: Deal with escapes and interpolation
    terminated(take_until(0.., quote), quote)
        .map(|value: &str| Token::String(value.into()))
        .parse_next(input)
}

fn number<'i>(input: &mut Located<&'i str>) -> PResult<Token<'i>> {
    // Optional sign
    let s = opt_sign.parse_next(input)?;

    // Integer and fractional parts
    let (i, f, d) = alt((
        // Integer part + optional fractional part
        seq!(dec_digits, opt(preceded('.', dec_digits))).map(|o| match o {
            ((i, _), Some((f, d))) => (i, f, d),
            ((i, _), None) => (i, 0, 0),
        }),
        // No integer part + required fractional part
        preceded('.', dec_digits).map(|(f, d)| (0, f, d)),
    ))
    .parse_next(input)?;

    // Exponent sign and exponent
    let (t, e) = opt(preceded(Caseless("e"), seq!(opt_sign, dec_digits)))
        .map(|o| match o {
            Some((t, (e, _))) => (t, e),
            None => (1, 0),
        })
        .parse_next(input)?;

    // See https://www.w3.org/TR/css-syntax-3/#convert-string-to-number
    let number =
        s as f32 * (i as f32 + f as f32 * 10f32.powi(-(d as i32))) * 10f32.powi(t * e as i32);

    Ok(Token::Number(number))
}

/// Parse an optional sign.
/// Returns -1 for '-', +1 for '+', and +1 otherwise.
fn opt_sign(input: &mut Located<&str>) -> PResult<i32> {
    alt(('+'.value(1), '-'.value(-1), empty.value(1))).parse_next(input)
}

/// Parses a string of decimal digits.
/// Returns the digits as an unsigned integer and the number of digits.
fn dec_digits(input: &mut Located<&str>) -> PResult<(u32, u32)> {
    take_while(1.., is_digit)
        .map(|digits: &str| (digits.parse().unwrap(), digits.len() as u32))
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let input = r#"
            ident ident-with-dash ident_with_underscore
            #hash #0ff
            // This is a comment
            "This is a string"
            123.45 15px 20%
            ( { [ ) } ]
        "#;
        assert_eq!(
            tokenize(input),
            Ok(vec![
                Token::Whitespace,
                Token::Ident("ident".into()),
                Token::Whitespace,
                Token::Ident("ident-with-dash".into()),
                Token::Whitespace,
                Token::Ident("ident_with_underscore".into()),
                Token::Whitespace,
                Token::Hash("hash".into()),
                Token::Whitespace,
                Token::Hash("0ff".into()),
                Token::Whitespace,
                Token::Comment(" This is a comment".into()),
                Token::Whitespace,
                Token::String("This is a string".into()),
                Token::Whitespace,
                Token::Number(123.45),
                Token::Whitespace,
                Token::Number(15.0),
                Token::Ident("px".into()),
                Token::Whitespace,
                Token::Number(20.0),
                Token::Symbol('%'),
                Token::Whitespace,
                Token::OpenDelim(Delim::Paren),
                Token::Whitespace,
                Token::OpenDelim(Delim::Brace),
                Token::Whitespace,
                Token::OpenDelim(Delim::Bracket),
                Token::Whitespace,
                Token::CloseDelim(Delim::Paren),
                Token::Whitespace,
                Token::CloseDelim(Delim::Brace),
                Token::Whitespace,
                Token::CloseDelim(Delim::Bracket),
                Token::Whitespace,
            ]),
        );
    }

    #[test]
    fn test_ident() {
        let input = Located::new("ident");
        let expected = Ok(Token::Ident("ident".into()));
        assert_eq!(ident.parse(input), expected);

        let input = Located::new("ident-with-dash");
        let expected = Ok(Token::Ident("ident-with-dash".into()));
        assert_eq!(ident.parse(input), expected);

        let input = Located::new("ident_with_underscore");
        let expected = Ok(Token::Ident("ident_with_underscore".into()));
        assert_eq!(ident.parse(input), expected);
    }
}
