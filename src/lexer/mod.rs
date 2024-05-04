use chumsky::prelude::*;
use std::borrow::Cow;

use crate::lexer::helpers::{is_digit, is_name, would_start_identifier};

mod helpers;

pub type Span = SimpleSpan<usize>;
pub type Err<'src> = extra::Err<Rich<'src, char, Span>>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Delim {
    Paren,
    Brace,
    Bracket,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'src> {
    Whitespace,
    Comment(Cow<'src, str>),
    Ident(Cow<'src, str>),
    Hash(Cow<'src, str>),
    String(Cow<'src, str>),
    Number(f32),
    Symbol(char),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenTree<'src> {
    Token(Token<'src>),
    Tree(Delim, Vec<(TokenTree<'src>, Span)>),
}

pub fn lexer<'src>() -> impl Parser<'src, &'src str, Vec<(TokenTree<'src>, Span)>, Err<'src>> {
    token_tree().repeated().collect()
}

fn token_tree<'src>() -> impl Parser<'src, &'src str, (TokenTree<'src>, Span), Err<'src>> {
    recursive(|token_tree| {
        choice((
            token_tree
                .clone()
                .repeated()
                .collect()
                .delimited_by(just('('), just(')'))
                .map(|tts| TokenTree::Tree(Delim::Paren, tts)),
            token_tree
                .clone()
                .repeated()
                .collect()
                .delimited_by(just('{'), just('}'))
                .map(|tts| TokenTree::Tree(Delim::Brace, tts)),
            token_tree
                .clone()
                .repeated()
                .collect()
                .delimited_by(just('['), just(']'))
                .map(|tts| TokenTree::Tree(Delim::Bracket, tts)),
            token().map(TokenTree::Token),
        ))
        .map_with(|tt, e| (tt, e.span()))
    })
}

fn token<'src>() -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    choice((
        text::whitespace().to(Token::Whitespace),
        line_comment(),
        block_comment(),
        ident(),
        hash(),
        string(),
        number(),
        any().map(Token::Symbol),
    ))
}

fn line_comment<'src>() -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    just("//")
        .ignore_then(any().and_is(just('\n').not()).repeated().to_slice())
        .map(|value: &str| Token::Comment(value.into()))
}

fn block_comment<'src>() -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    just("/*")
        .ignore_then(
            any()
                .and_is(just("*/").not())
                .repeated()
                .to_slice()
                .then_ignore(just("*/").repeated().at_least(0).at_most(1)),
        )
        .map(|value: &str| Token::Comment(value.into()))
}

fn ident<'src>() -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    peek_ident_start()
        .ignore_then(ident_sequence())
        .map(|value| Token::Ident(value.into()))
}

fn peek_ident_start<'src>() -> impl Parser<'src, &'src str, (), Err<'src>> + Clone {
    custom(|input| {
        if would_start_identifier(input.slice_from(input.offset()..)) {
            Ok(())
        } else {
            Err(Rich::custom(
                input.span_since(input.offset()),
                "expected identifier",
            ))
        }
    })
}

fn ident_sequence<'src>() -> impl Parser<'src, &'src str, &'src str, Err<'src>> + Clone {
    any().validate(|c, _, _| is_name(c)).repeated().to_slice()
}

fn hash<'src>() -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    just('#')
        .ignore_then(ident_sequence())
        .map(|value: &str| Token::Hash(value.into()))
}

fn string<'src>() -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    choice((string_with_quote('"'), string_with_quote('\'')))
}

fn string_with_quote<'src>(
    quote: char,
) -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    // TODO: Deal with escapes and interpolation
    just(quote)
        .ignore_then(none_of([quote]))
        .then_ignore(just(quote))
        .to_slice()
        .map(|value: &str| Token::String(value.into()))
}

fn number<'src>() -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    todo()
    // // Optional sign
    // let s = opt_sign.parse_next(input)?;
    //
    // // Integer and fractional parts
    // let (i, f, d) = alt((
    //     // Integer part + optional fractional part
    //     seq!(dec_digits, opt(preceded('.', dec_digits))).map(|o| match o {
    //         ((i, _), Some((f, d))) => (i, f, d),
    //         ((i, _), None) => (i, 0, 0),
    //     }),
    //     // No integer part + required fractional part
    //     preceded('.', dec_digits).map(|(f, d)| (0, f, d)),
    // ))
    // .parse_next(input)?;
    //
    // // Exponent sign and exponent
    // let (t, e) = opt(preceded(Caseless("e"), seq!(opt_sign, dec_digits)))
    //     .map(|o| match o {
    //         Some((t, (e, _))) => (t, e),
    //         None => (1, 0),
    //     })
    //     .parse_next(input)?;
    //
    // // See https://www.w3.org/TR/css-syntax-3/#convert-string-to-number
    // let number =
    //     s as f32 * (i as f32 + f as f32 * 10f32.powi(-(d as i32))) * 10f32.powi(t * e as i32);
    //
    // Ok(Token::Number(number))
}

/// Parse an optional sign.
/// Returns -1 for '-', +1 for '+', and +1 otherwise.
fn opt_sign<'src>() -> impl Parser<'src, &'src str, i32, Err<'src>> + Clone {
    choice((just('+').to(1), just('-').to(-1), empty().to(1)))
}

/// Parses a string of decimal digits.
/// Returns the digits as an unsigned integer and the number of digits.
fn dec_digits<'src>() -> impl Parser<'src, &'src str, (u32, u32), Err<'src>> + Clone {
    text::digits(10)
        .to_slice()
        .map(|digits: &str| (digits.parse().unwrap(), digits.len() as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! token {
        ($($tt:tt)*) => {
            TokenTree::Token(Token::$($tt)*)
        };
    }
    macro_rules! tree {
        ($delim:ident, [$($tokens:tt)*]) => {
            TokenTree::Tree(Delim::$delim, vec![$($tokens)*])
        };
    }

    #[test]
    fn test_line_comment() {
        let input = "// This is a comment\n";
        let expected = Ok(Token::Comment(" This is a comment".into()));
        assert_eq!(line_comment().lazy().parse(input).into_result(), expected);

        let input = "// This is a comment";
        let expected = Ok(Token::Comment(" This is a comment".into()));
        assert_eq!(line_comment().lazy().parse(input).into_result(), expected);
    }

    #[test]
    fn test_block_comment() {
        let input = "/* This is a comment */";
        let expected = Ok(Token::Comment(" This is a comment ".into()));
        assert_eq!(block_comment().lazy().parse(input).into_result(), expected);

        let input = "/* This is a comment";
        let expected = Ok(Token::Comment(" This is a comment".into()));
        assert_eq!(block_comment().lazy().parse(input).into_result(), expected);
    }

    // #[test]
    // fn test_tokenize() {
    //     let input = r#"
    //         ident ident-with-dash ident_with_underscore
    //         #hash #0ff
    //         // This is a comment
    //         "This is a string"
    //         123.45 15px 20%
    //         (paren) { brace} [bracket ]
    //     "#;
    //     println!("{:?}", lexer().parse(input));
    //
    //     // assert_eq!(
    //     //     tokenize(input),
    //     //     Ok(vec![
    //     //         token!(Whitespace),
    //     //         token!(Ident("ident".into())),
    //     //         token!(Whitespace),
    //     //         token!(Ident("ident-with-dash".into())),
    //     //         token!(Whitespace),
    //     //         token!(Ident("ident_with_underscore".into())),
    //     //         token!(Whitespace),
    //     //         token!(Hash("hash".into())),
    //     //         token!(Whitespace),
    //     //         token!(Hash("0ff".into())),
    //     //         token!(Whitespace),
    //     //         token!(Comment(" This is a comment".into())),
    //     //         token!(Whitespace),
    //     //         token!(String("This is a string".into())),
    //     //         token!(Whitespace),
    //     //         token!(Number(123.45)),
    //     //         token!(Whitespace),
    //     //         token!(Number(15.0)),
    //     //         token!(Ident("px".into())),
    //     //         token!(Whitespace),
    //     //         token!(Number(20.0)),
    //     //         token!(Symbol('%')),
    //     //         token!(Whitespace),
    //     //         tree!(Paren, [token!(Ident("paren".into())),]),
    //     //         token!(Whitespace),
    //     //         tree!(Brace, [token!(Whitespace), token!(Ident("brace".into())),]),
    //     //         token!(Whitespace),
    //     //         tree!(
    //     //             Bracket,
    //     //             [token!(Ident("bracket".into())), token!(Whitespace),]
    //     //         ),
    //     //         token!(Whitespace),
    //     //     ]),
    //     // );
    // }

    // #[test]
    // fn test_ident() {
    //     let input = Located::new("ident");
    //     let expected = Ok(Token::Ident("ident".into()));
    //     assert_eq!(ident.parse(input), expected);
    //
    //     let input = Located::new("ident-with-dash");
    //     let expected = Ok(Token::Ident("ident-with-dash".into()));
    //     assert_eq!(ident.parse(input), expected);
    //
    //     let input = Located::new("ident_with_underscore");
    //     let expected = Ok(Token::Ident("ident_with_underscore".into()));
    //     assert_eq!(ident.parse(input), expected);
    // }
    //
    // #[test]
    // fn test_comment() {
    //     let input = "// This is a comment\n";
    //     let expected = Ok(vec![
    //         token!(Comment(" This is a comment".into())),
    //         token!(Whitespace),
    //     ]);
    //     assert_eq!(tokenize(input), expected);
    //
    //     let input = "// This is a comment";
    //     let expected = Ok(vec![token!(Comment(" This is a comment".into()))]);
    //     assert_eq!(tokenize(input), expected);
    //
    //     let input = "/* This is a comment */";
    //     let expected = Ok(vec![token!(Comment(" This is a comment ".into()))]);
    //     assert_eq!(tokenize(input), expected);
    //
    //     let input = "/* This is a comment";
    //     assert!(tokenize(input).is_err());
    // }
    //
    // #[test]
    // fn test_string() {
    //     let input = r#""This is a string""#;
    //     let expected = Ok(vec![token!(String("This is a string".into()))]);
    //     assert_eq!(tokenize(input), expected);
    //
    //     let input = r#"'This is a string'"#;
    //     let expected = Ok(vec![token!(String("This is a string".into()))]);
    //     assert_eq!(tokenize(input), expected);
    //
    //     let input = r#""This is a string"#;
    //     assert!(tokenize(input).is_err());
    // }
}
