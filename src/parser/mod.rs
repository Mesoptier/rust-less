use std::borrow::Cow;

use winnow::combinator::{alt, cut_err, eof, opt, preceded, repeat, repeat_till, terminated};
use winnow::error::StrContext;
use winnow::token::{any, one_of};
use winnow::{seq, PResult, Parser};

use crate::ast::{Item, Stylesheet};
use crate::lexer::{Delim, Token, TokenTree};
use crate::ref_stream::RefStream;

type TokenStream<'t, 'i> = RefStream<'t, TokenTree<'i>>;

fn whitespace_or_comment<'t, 'i>(input: &mut TokenStream<'t, 'i>) -> PResult<&'t TokenTree<'i>> {
    one_of(|tt| matches!(tt, &TokenTree::Token(Token::Whitespace | Token::Comment(_))))
        .parse_next(input)
}

/// Consume any number of whitespace or comments.
fn whitespace(input: &mut TokenStream) -> PResult<()> {
    repeat(0.., whitespace_or_comment).parse_next(input)
}

fn symbol<'i>(c: char) -> impl FnMut(&mut TokenStream<'_, 'i>) -> PResult<()> {
    move |input| {
        any.verify_map(|tt| match tt {
            &TokenTree::Token(Token::Symbol(s)) if s == c => Some(()),
            _ => None,
        })
        .parse_next(input)
    }
}

fn ident<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Cow<'i, str>> {
    any.verify_map(|tt: &'_ TokenTree<'i>| match tt {
        TokenTree::Token(Token::Ident(ident)) => Some(ident.clone()),
        _ => None,
    })
    .parse_next(input)
}

fn simple_block<'i>(
    delim: Delim,
) -> impl FnMut(&mut TokenStream<'_, 'i>) -> PResult<Vec<TokenTree<'i>>> {
    move |input| {
        any.verify_map(|tt: &'_ TokenTree<'i>| match tt {
            TokenTree::Delim(d, tokens) if *d == delim => Some(tokens.clone()),
            _ => None,
        })
        .parse_next(input)
    }
}

fn guarded_block<'i>(
    input: &mut TokenStream<'_, 'i>,
) -> PResult<(Option<Vec<TokenTree<'i>>>, Vec<TokenTree<'i>>)> {
    seq!(
        // Guard
        opt(preceded(
            (
                whitespace,
                any.verify(
                    |tt| matches!(tt, TokenTree::Token(Token::Ident(ident)) if ident == "when")
                ),
                whitespace,
            ),
            cut_err(simple_block(Delim::Paren)),
        )),
        // Block
        preceded(whitespace, simple_block(Delim::Brace)),
    )
    .parse_next(input)
}

fn component_value<'t, 'i>(input: &mut TokenStream<'t, 'i>) -> PResult<&'t TokenTree<'i>> {
    any.parse_next(input)
}

pub fn stylesheet<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Stylesheet<'i>> {
    preceded(
        whitespace,
        repeat_till(0.., cut_err(terminated(item, whitespace)), eof),
    )
    .map(|(items, _)| Stylesheet { items })
    .parse_next(input)
}

fn item<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    alt((
        item_variable_declaration.context(StrContext::Label("item_variable_declaration")),
        item_variable_call.context(StrContext::Label("item_variable_call")),
        item_at_rule.context(StrContext::Label("item_at_rule")),
        item_mixin_rule.context(StrContext::Label("item_mixin_rule")),
        item_qualified_rule.context(StrContext::Label("item_qualified_rule")),
        item_declaration.context(StrContext::Label("item_declaration")),
        item_mixin_call.context(StrContext::Label("item_mixin_call")),
    ))
    .parse_next(input)
}

fn item_at_rule<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    seq!(
        _: symbol('@'),
        ident,
        repeat_till(0.., any.map(Clone::clone), preceded(whitespace, alt((
            eof.value(None),
            symbol(';').value(None),
            simple_block(Delim::Brace).map(Some),
        )))),
    )
    .map(|(name, (prelude, block))| Item::AtRule {
        name,
        prelude,
        block,
    })
    .parse_next(input)
}

fn item_mixin_rule<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    seq!(
        _: symbol('.'),
        ident,
        simple_block(Delim::Paren),
        _: whitespace,
        guarded_block,
    )
    .map(|(name, arguments, (guard, block))| Item::MixinRule {
        name,
        arguments,
        guard,
        block,
    })
    .parse_next(input)
}

fn item_qualified_rule<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    repeat_till(1.., component_value.map(Clone::clone), guarded_block)
        .map(|(selectors, (guard, block))| Item::QualifiedRule {
            selectors,
            guard,
            block,
        })
        .parse_next(input)
}

// TODO: https://drafts.csswg.org/css-syntax/#consume-declaration
fn item_declaration<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    seq!(
        repeat_till(
            1..,
            component_value.map(Clone::clone),
            (whitespace, symbol(':'), whitespace)
        ),
        repeat_till(
            1..,
            component_value.map(Clone::clone),
            (whitespace, alt((symbol(';'), eof.void())))
        ),
    )
    .map(|((name, _), (value, _))| {
        let mut value: Vec<TokenTree> = value;
        let mut important = false;

        // Parse `!important` flag.
        if value.ends_with(&[
            TokenTree::Token(Token::Symbol('!')),
            TokenTree::Token(Token::Ident("important".into())),
        ]) {
            important = true;
            value.pop();
            value.pop();
        }

        // Remove trailing whitespace or comments.
        while let Some(TokenTree::Token(Token::Whitespace | Token::Comment(_))) = value.last() {
            value.pop();
        }

        Item::Declaration {
            name,
            value,
            important,
        }
    })
    .parse_next(input)
}

fn item_variable_declaration<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    seq!(
        _: symbol('@'),
        ident,
        _: (whitespace, symbol(':'), whitespace),
        repeat_till(0.., any.map(Clone::clone), (whitespace, symbol(';'))),
    )
    .map(|(name, (value, _))| Item::VariableDeclaration { name, value })
    .parse_next(input)
}

fn item_variable_call<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    seq!(_: symbol('@'), ident, simple_block(Delim::Paren), _: (whitespace, symbol(';')))
        .map(|(name, arguments)| Item::VariableCall { name, arguments })
        .parse_next(input)
}

fn item_mixin_call<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use crate::lexer::tokenize;

    use super::*;

    macro_rules! assert_parse_ok {
        ($input:expr, $expected:expr) => {
            let tokens = tokenize($input).unwrap();
            let mut input = RefStream::new(&tokens);
            let result = item(&mut input);
            assert_eq!(result, Ok($expected));
            assert_eq!(input.into_inner(), &[]);
        };
    }

    #[test]
    fn test_variable_declaration() {
        assert_parse_ok!(
            "@foo: bar, baz;",
            Item::VariableDeclaration {
                name: "foo".into(),
                value: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
            }
        );
    }

    #[test]
    fn test_variable_call() {
        assert_parse_ok!(
            "@foo(bar, baz);",
            Item::VariableCall {
                name: "foo".into(),
                arguments: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
            }
        );
    }

    #[test]
    fn test_mixin_rule() {
        assert_parse_ok!(
            ".foo(bar, baz) { }",
            Item::MixinRule {
                name: "foo".into(),
                arguments: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
                guard: None,
                block: vec![TokenTree::Token(Token::Whitespace)],
            }
        );

        assert_parse_ok!(
            ".foo(bar, baz) when (true) { }",
            Item::MixinRule {
                name: "foo".into(),
                arguments: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
                guard: Some(vec![TokenTree::Token(Token::Ident("true".into()))]),
                block: vec![TokenTree::Token(Token::Whitespace)],
            }
        );
    }

    #[test]
    fn test_qualified_rule() {
        assert_parse_ok!(
            "foo { }",
            Item::QualifiedRule {
                selectors: vec![TokenTree::Token(Token::Ident("foo".into()))],
                guard: None,
                block: vec![TokenTree::Token(Token::Whitespace)],
            }
        );
    }

    #[test]
    fn test_at_rule() {
        assert_parse_ok!(
            "@foo;",
            Item::AtRule {
                name: "foo".into(),
                prelude: vec![],
                block: None,
            }
        );

        assert_parse_ok!(
            "@foo bar;",
            Item::AtRule {
                name: "foo".into(),
                prelude: vec![
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("bar".into()))
                ],
                block: None,
            }
        );

        assert_parse_ok!(
            "@foo { }",
            Item::AtRule {
                name: "foo".into(),
                prelude: vec![],
                block: Some(vec![TokenTree::Token(Token::Whitespace)]),
            }
        );
    }

    #[test]
    fn test_declaration() {
        assert_parse_ok!(
            "foo: bar;",
            Item::Declaration {
                name: vec![TokenTree::Token(Token::Ident("foo".into()))],
                value: vec![TokenTree::Token(Token::Ident("bar".into()))],
                important: false,
            }
        );

        assert_parse_ok!(
            "foo: bar !important;",
            Item::Declaration {
                name: vec![TokenTree::Token(Token::Ident("foo".into()))],
                value: vec![TokenTree::Token(Token::Ident("bar".into()))],
                important: true,
            }
        );
    }

    #[test]
    fn test_mixin_call() {
        assert_parse_ok!(
            ".foo(bar, baz);",
            Item::MixinCall {
                selector: vec![TokenTree::Token(Token::Ident("foo".into()))],
                arguments: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
            }
        );

        assert_parse_ok!(
            "#namespace.foo(bar, baz)",
            Item::MixinCall {
                selector: vec![
                    TokenTree::Token(Token::Hash("namespace".into())),
                    TokenTree::Token(Token::Symbol('.')),
                    TokenTree::Token(Token::Ident("foo".into())),
                ],
                arguments: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
            }
        );
    }
}
