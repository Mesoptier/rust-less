use std::borrow::Cow;

use chumsky::prelude::*;

use crate::lexer::helpers::{is_name, would_start_identifier};

mod helpers;

pub type Span = SimpleSpan<usize>;
pub type Spanned<T> = (T, Span);
pub type Err<'src> = extra::Err<Rich<'src, char, Span>>;

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
    Tree(Delim, Vec<Spanned<TokenTree<'src>>>),
}

pub fn lexer<'src>() -> impl Parser<'src, &'src str, Vec<Spanned<TokenTree<'src>>>, Err<'src>> {
    token_tree().repeated().collect()
}

fn token_tree<'src>() -> impl Parser<'src, &'src str, Spanned<TokenTree<'src>>, Err<'src>> {
    recursive(|token_tree| {
        choice((
            tree(Delim::Paren, token_tree.clone()),
            tree(Delim::Brace, token_tree.clone()),
            tree(Delim::Bracket, token_tree.clone()),
            token().map(TokenTree::Token),
        ))
        .map_with(|tt, e| (tt, e.span()))
    })
}

fn tree<'src>(
    delim: Delim,
    token_tree: impl Parser<'src, &'src str, Spanned<TokenTree<'src>>, Err<'src>> + Clone,
) -> impl Parser<'src, &'src str, TokenTree<'src>, Err<'src>> + Clone {
    just(delim.open())
        .ignore_then(
            // TODO: Clean this up? Test for close delimiter before trying to parse token_tree?
            token_tree
                .and_is(just(delim.close()).not())
                .repeated()
                .collect()
                .map(move |tts| TokenTree::Tree(delim, tts)),
        )
        .then_ignore(
            just(delim.close()), // TODO: error recovery
        )
}

fn token<'src>() -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    choice((
        text::whitespace().at_least(1).to(Token::Whitespace),
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
        .ignore_then(any().and_is(just("*/").not()).repeated().to_slice())
        .then_ignore(choice((just("*/").ignored(), end())))
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
    any().filter(|c: &char| is_name(*c)).repeated().to_slice()
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
        .ignore_then(any().and_is(just(quote).not()).repeated().to_slice())
        .then_ignore(just(quote))
        .map(|value: &str| Token::String(value.into()))
}

fn number<'src>() -> impl Parser<'src, &'src str, Token<'src>, Err<'src>> + Clone {
    group((
        // Optional sign
        opt_sign(),
        // Integer and fractional parts
        choice((
            // Integer part + optional fractional part
            group((dec_digits(), just('.').ignore_then(dec_digits()).or_not())).map(|o| match o {
                ((i, _), Some((f, d))) => (i, f, d),
                ((i, _), None) => (i, 0, 0),
            }),
            // No integer part + required fractional part
            just('.').ignore_then(dec_digits()).map(|(f, d)| (0, f, d)),
        )),
        // Exponent sign and exponent
        one_of("eE")
            .ignore_then(opt_sign().then(dec_digits()))
            .or_not()
            .map(|o| match o {
                Some((t, (e, _))) => (t, e),
                None => (1, 0),
            }),
    ))
    .map(|(s, (i, f, d), (t, e))| {
        // See https://www.w3.org/TR/css-syntax-3/#convert-string-to-number
        let number =
            s as f32 * (i as f32 + f as f32 * 10f32.powi(-(d as i32))) * 10f32.powi(t * e as i32);

        Token::Number(number)
    })
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

    #[test]
    fn test_line_comment() {
        let input = "// This is a comment\n";
        let expected = Ok(Token::Comment(" This is a comment".into()));
        assert_eq!(line_comment().lazy().parse(input).into_result(), expected);

        let input = "// This is a comment";
        let expected = Ok(Token::Comment(" This is a comment".into()));
        assert_eq!(line_comment().parse(input).into_result(), expected);
    }

    #[test]
    fn test_block_comment() {
        let input = "/* This is a comment */";
        let expected = Ok(Token::Comment(" This is a comment ".into()));
        assert_eq!(block_comment().parse(input).into_result(), expected);

        let input = "/* This is a comment";
        let expected = Ok(Token::Comment(" This is a comment".into()));
        assert_eq!(block_comment().parse(input).into_result(), expected);
    }

    #[test]
    fn test_ident() {
        let input = "ident";
        let expected = Ok(Token::Ident("ident".into()));
        assert_eq!(ident().parse(input).into_result(), expected);

        let input = "ident-with-dash";
        let expected = Ok(Token::Ident("ident-with-dash".into()));
        assert_eq!(ident().parse(input).into_result(), expected);

        let input = "ident_with_underscore";
        let expected = Ok(Token::Ident("ident_with_underscore".into()));
        assert_eq!(ident().parse(input).into_result(), expected);

        let input = "--ident";
        let expected = Ok(Token::Ident("--ident".into()));
        assert_eq!(ident().parse(input).into_result(), expected);

        let input = "--0ident";
        let expected = Ok(Token::Ident("--0ident".into()));
        assert_eq!(ident().parse(input).into_result(), expected);

        let input = "-ident";
        let expected = Ok(Token::Ident("-ident".into()));
        assert_eq!(ident().parse(input).into_result(), expected);

        let input = "-0ident";
        assert!(ident().parse(input).has_errors());

        let input = "ident not-parsed";
        let expected = Ok(Token::Ident("ident".into()));
        assert_eq!(ident().lazy().parse(input).into_result(), expected);
    }

    #[test]
    fn test_hash() {
        let input = "#hash";
        let expected = Ok(Token::Hash("hash".into()));
        assert_eq!(hash().parse(input).into_result(), expected);

        let input = "#0ff";
        let expected = Ok(Token::Hash("0ff".into()));
        assert_eq!(hash().parse(input).into_result(), expected);
    }

    #[test]
    fn test_string() {
        let input = r#""This is a string""#;
        let expected = Ok(Token::String("This is a string".into()));
        assert_eq!(string().parse(input).into_result(), expected);

        let input = r#"'This is a string'"#;
        let expected = Ok(Token::String("This is a string".into()));
        assert_eq!(string().parse(input).into_result(), expected);

        let input = r#""This is a string"#;
        assert!(string().parse(input).has_errors());
    }

    #[test]
    fn test_number() {
        let input = "123.45";
        let expected = Ok(Token::Number(123.45));
        assert_eq!(number().parse(input).into_result(), expected);

        let input = "15px";
        let expected = Ok(Token::Number(15.0));
        assert_eq!(number().lazy().parse(input).into_result(), expected);

        let input = "20%";
        let expected = Ok(Token::Number(20.0));
        assert_eq!(number().lazy().parse(input).into_result(), expected);
    }

    #[test]
    fn test_tokenize() {
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

        let input = r#"
            ident ident-with-dash ident_with_underscore
            #hash #0ff
            // This is a comment
            "This is a string"
            123.45 15px 20%
            (paren) { brace} [bracket ]
        "#;
        assert_eq!(
            lexer().parse(input).into_result(),
            Ok(vec![
                (token!(Whitespace), Span::new(0, 13)),
                (token!(Ident("ident".into())), Span::new(13, 18)),
                (token!(Whitespace), Span::new(18, 19)),
                (token!(Ident("ident-with-dash".into())), Span::new(19, 34)),
                (token!(Whitespace), Span::new(34, 35)),
                (
                    token!(Ident("ident_with_underscore".into())),
                    Span::new(35, 56),
                ),
                (token!(Whitespace), Span::new(56, 69)),
                (token!(Hash("hash".into())), Span::new(69, 74)),
                (token!(Whitespace), Span::new(74, 75)),
                (token!(Hash("0ff".into())), Span::new(75, 79)),
                (token!(Whitespace), Span::new(79, 92)),
                (
                    token!(Comment(" This is a comment".into())),
                    Span::new(92, 112),
                ),
                (token!(Whitespace), Span::new(112, 125)),
                (
                    token!(String("This is a string".into())),
                    Span::new(125, 143),
                ),
                (token!(Whitespace), Span::new(143, 156)),
                (token!(Number(123.45)), Span::new(156, 162)),
                (token!(Whitespace), Span::new(162, 163)),
                (token!(Number(15.0)), Span::new(163, 165)),
                (token!(Ident("px".into())), Span::new(165, 167)),
                (token!(Whitespace), Span::new(167, 168)),
                (token!(Number(20.0)), Span::new(168, 170)),
                (token!(Symbol('%')), Span::new(170, 171)),
                (token!(Whitespace), Span::new(171, 184)),
                (
                    tree!(
                        Paren,
                        [(token!(Ident("paren".into())), Span::new(185, 190))]
                    ),
                    Span::new(184, 191),
                ),
                (token!(Whitespace), Span::new(191, 192)),
                (
                    tree!(
                        Brace,
                        [
                            (token!(Whitespace), Span::new(193, 194)),
                            (token!(Ident("brace".into())), Span::new(194, 199)),
                        ]
                    ),
                    Span::new(192, 200),
                ),
                (token!(Whitespace), Span::new(200, 201)),
                (
                    tree!(
                        Bracket,
                        [
                            (token!(Ident("bracket".into())), Span::new(202, 209)),
                            (token!(Whitespace), Span::new(209, 210)),
                        ]
                    ),
                    Span::new(201, 211),
                ),
                (token!(Whitespace), Span::new(211, 220)),
            ])
        );
    }
}
