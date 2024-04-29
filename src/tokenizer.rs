use std::borrow::Cow;

use winnow::ascii::Caseless;
use winnow::combinator::{alt, cut_err, empty, fail, opt, peek, preceded, repeat, terminated};
use winnow::stream::AsChar;
use winnow::token::{any, one_of, take_until, take_while};
use winnow::{dispatch, seq, Located, PResult, Parser};

use crate::lexer::helpers::{is_digit, is_name, would_start_identifier};

type Stream<'i> = Located<&'i str>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Delim {
    Paren,
    Brace,
    Bracket,
}

impl Delim {
    pub const fn open(&self) -> char {
        match self {
            Delim::Paren => '(',
            Delim::Brace => '{',
            Delim::Bracket => '[',
        }
    }

    pub const fn close(&self) -> char {
        match self {
            Delim::Paren => ')',
            Delim::Brace => '}',
            Delim::Bracket => ']',
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'i> {
    Whitespace,
    Comment(Cow<'i, str>),
    Ident(Cow<'i, str>),
    Hash(Cow<'i, str>),
    String(Cow<'i, str>),
    Number(f32),
    Symbol(char),
}

type TokenStream<'i> = Vec<TokenTree<'i>>;

#[derive(Clone, Debug, PartialEq)]
enum TokenTree<'i> {
    Token(Token<'i>),
    Delim(Delim, TokenStream<'i>),
}

pub fn tokenize(input: &str) -> Result<TokenStream, String> {
    repeat(0.., token_tree)
        .parse(Stream::new(input))
        .map_err(|e| e.to_string())
}

fn token_tree<'i>(input: &mut Stream<'i>) -> PResult<TokenTree<'i>> {
    dispatch!(peek(any);
        '(' => delim(Delim::Paren),
        '{' => delim(Delim::Brace),
        '[' => delim(Delim::Bracket),
        ')' | '}' | ']' => fail,
        _ => token.map(TokenTree::Token),
    )
    .parse_next(input)
}

fn delim<'i>(delim: Delim) -> impl FnMut(&mut Stream<'i>) -> PResult<TokenTree<'i>> {
    move |input| {
        preceded(
            delim.open(),
            cut_err(terminated(repeat(0.., token_tree), delim.close())),
        )
        .map(|tokens| TokenTree::Delim(delim, tokens))
        .parse_next(input)
    }
}

fn token<'i>(input: &mut Stream<'i>) -> PResult<Token<'i>> {
    alt((
        whitespace,
        line_comment,
        block_comment,
        ident,
        hash,
        string,
        number,
        any.map(Token::Symbol),
    ))
    .parse_next(input)
}

fn whitespace<'i>(input: &mut Stream<'i>) -> PResult<Token<'i>> {
    take_while(1.., char::is_whitespace)
        .value(Token::Whitespace)
        .parse_next(input)
}

fn line_comment<'i>(input: &mut Stream<'i>) -> PResult<Token<'i>> {
    preceded("//", take_while(0.., |c: char| !c.is_newline()))
        .map(|value: &str| Token::Comment(value.into()))
        .parse_next(input)
}

fn block_comment<'i>(input: &mut Stream<'i>) -> PResult<Token<'i>> {
    "/*".parse_next(input)?;
    cut_err(terminated(take_until(0.., "*/"), "*/"))
        .map(|value: &str| Token::Comment(value.into()))
        .parse_next(input)
}

fn ident<'i>(input: &mut Stream<'i>) -> PResult<Token<'i>> {
    preceded(peek_ident_start, ident_sequence)
        .map(|value| Token::Ident(value.into()))
        .parse_next(input)
}

fn ident_sequence<'i>(input: &mut Stream<'i>) -> PResult<&'i str> {
    take_while(1.., is_name).parse_next(input)
}

/// Matches if the next characters would start an identifier.
fn peek_ident_start<'i>(input: &mut Stream<'i>) -> PResult<()> {
    if would_start_identifier(input.as_ref()) {
        empty.parse_next(input)
    } else {
        fail.parse_next(input)
    }
}

fn hash<'i>(input: &mut Stream<'i>) -> PResult<Token<'i>> {
    preceded('#', ident_sequence)
        .map(|value: &str| Token::Hash(value.into()))
        .parse_next(input)
}

fn string<'i>(input: &mut Stream<'i>) -> PResult<Token<'i>> {
    let quote = one_of(|c| c == '"' || c == '\'').parse_next(input)?;
    // TODO: Deal with escapes and interpolation
    cut_err(terminated(take_until(0.., quote), quote))
        .map(|value: &str| Token::String(value.into()))
        .parse_next(input)
}

fn number<'i>(input: &mut Stream<'i>) -> PResult<Token<'i>> {
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
fn opt_sign(input: &mut Stream) -> PResult<i32> {
    alt(('+'.value(1), '-'.value(-1), empty.value(1))).parse_next(input)
}

/// Parses a string of decimal digits.
/// Returns the digits as an unsigned integer and the number of digits.
fn dec_digits(input: &mut Stream) -> PResult<(u32, u32)> {
    take_while(1.., is_digit)
        .map(|digits: &str| (digits.parse().unwrap(), digits.len() as u32))
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! token {
        ($($tt:tt)*) => {
            TokenTree::Token(Token::$($tt)*)
        };
    }
    macro_rules! delim {
        ($delim:ident, [$($tokens:tt)*]) => {
            TokenTree::Delim(Delim::$delim, vec![$($tokens)*])
        };
    }

    #[test]
    fn test_tokenize() {
        let input = r#"
            ident ident-with-dash ident_with_underscore
            #hash #0ff
            // This is a comment
            "This is a string"
            123.45 15px 20%
            (paren) { brace} [bracket ]
        "#;
        assert_eq!(
            tokenize(input),
            Ok(vec![
                token!(Whitespace),
                token!(Ident("ident".into())),
                token!(Whitespace),
                token!(Ident("ident-with-dash".into())),
                token!(Whitespace),
                token!(Ident("ident_with_underscore".into())),
                token!(Whitespace),
                token!(Hash("hash".into())),
                token!(Whitespace),
                token!(Hash("0ff".into())),
                token!(Whitespace),
                token!(Comment(" This is a comment".into())),
                token!(Whitespace),
                token!(String("This is a string".into())),
                token!(Whitespace),
                token!(Number(123.45)),
                token!(Whitespace),
                token!(Number(15.0)),
                token!(Ident("px".into())),
                token!(Whitespace),
                token!(Number(20.0)),
                token!(Symbol('%')),
                token!(Whitespace),
                delim!(Paren, [token!(Ident("paren".into())),]),
                token!(Whitespace),
                delim!(Brace, [token!(Whitespace), token!(Ident("brace".into())),]),
                token!(Whitespace),
                delim!(
                    Bracket,
                    [token!(Ident("bracket".into())), token!(Whitespace),]
                ),
                token!(Whitespace),
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

    #[test]
    fn test_comment() {
        let input = "// This is a comment\n";
        let expected = Ok(vec![
            token!(Comment(" This is a comment".into())),
            token!(Whitespace),
        ]);
        assert_eq!(tokenize(input), expected);

        let input = "// This is a comment";
        let expected = Ok(vec![token!(Comment(" This is a comment".into()))]);
        assert_eq!(tokenize(input), expected);

        let input = "/* This is a comment */";
        let expected = Ok(vec![token!(Comment(" This is a comment ".into()))]);
        assert_eq!(tokenize(input), expected);

        let input = "/* This is a comment";
        assert!(tokenize(input).is_err());
    }

    #[test]
    fn test_string() {
        let input = r#""This is a string""#;
        let expected = Ok(vec![token!(String("This is a string".into()))]);
        assert_eq!(tokenize(input), expected);

        let input = r#"'This is a string'"#;
        let expected = Ok(vec![token!(String("This is a string".into()))]);
        assert_eq!(tokenize(input), expected);

        let input = r#""This is a string"#;
        assert!(tokenize(input).is_err());
    }

    #[test]
    fn print_file() {
        let path = std::path::Path::new("node_modules/@less/test-data/less/_main/calc.less");
        let file = std::fs::read_to_string(path).unwrap();
        let tokens = tokenize(&file).unwrap();
        for token in tokens {
            println!("{:?}", token);
        }
    }
}
