use std::borrow::Cow;

use winnow::combinator::{alt, cut_err, eof, opt, preceded, repeat, repeat_till, terminated};
use winnow::token::{any, one_of};
use winnow::{seq, PResult, Parser};

use crate::ast::{Item, Stylesheet};
use crate::lexer::{Delim, Token, TokenTree};

type TokenStream<'t, 'i> = &'t [TokenTree<'i>];

/// Consume any whitespace or comments.
fn whitespace(input: &mut TokenStream) -> PResult<()> {
    repeat(
        0..,
        one_of(|tt| matches!(tt, TokenTree::Token(Token::Whitespace | Token::Comment(_)))),
    )
    .parse_next(input)
}

fn symbol<'i>(c: char) -> impl FnMut(&mut TokenStream<'_, 'i>) -> PResult<()> {
    move |input| {
        any.verify_map(|tt| match tt {
            TokenTree::Token(Token::Symbol(s)) if s == c => Some(()),
            _ => None,
        })
        .parse_next(input)
    }
}

fn ident<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Cow<'i, str>> {
    any.verify_map(|tt| match tt {
        TokenTree::Token(Token::Ident(ident)) => Some(ident),
        _ => None,
    })
    .parse_next(input)
}

fn simple_block<'i>(
    delim: Delim,
) -> impl FnMut(&mut TokenStream<'_, 'i>) -> PResult<Vec<TokenTree<'i>>> {
    move |input| {
        any.verify_map(|tt| match tt {
            // TODO: Shouldn't we be cloning the tokens here? I guess winnow is already cloning somewhere?
            TokenTree::Delim(d, tokens) if d == delim => Some(tokens),
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
        cut_err(preceded(whitespace, simple_block(Delim::Brace))),
    )
    .parse_next(input)
}

fn component_value<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<TokenTree<'i>> {
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
        item_variable_declaration,
        item_variable_call,
        // item_at_rule,
        item_mixin_rule,
        item_qualified_rule,
        // item_declaration,
        // item_mixin_call,
    ))
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
    repeat_till(1.., component_value, guarded_block)
        .map(|(selectors, (guard, block))| Item::QualifiedRule {
            selectors,
            guard,
            block,
        })
        .parse_next(input)
}

fn item_variable_declaration<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    seq!(
        _: symbol('@'),
        ident,
        _: (whitespace, symbol(':'), whitespace),
        repeat_till(0.., any, (whitespace, symbol(';'))),
    )
    .map(|(name, (value, _))| Item::VariableDeclaration { name, value })
    .parse_next(input)
}

fn item_variable_call<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    seq!(_: symbol('@'), ident, simple_block(Delim::Paren), _: (whitespace, symbol(';')))
        .map(|(name, arguments)| Item::VariableCall { name, arguments })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use crate::lexer::tokenize;

    use super::*;

    #[test]
    fn test_variable_declaration() {
        let input = "@foo: bar, baz;";
        let tokens = tokenize(input).unwrap();
        let mut input = &tokens[..];
        let result = item_variable_declaration(&mut input);
        assert_eq!(
            result,
            Ok(Item::VariableDeclaration {
                name: "foo".into(),
                value: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
            })
        );
        assert_eq!(input, &[]);
    }

    #[test]
    fn test_variable_call() {
        let input = "@foo(bar, baz);";
        let tokens = tokenize(input).unwrap();
        let mut input = &tokens[..];
        let result = item_variable_call(&mut input);
        assert_eq!(
            result,
            Ok(Item::VariableCall {
                name: "foo".into(),
                arguments: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
            })
        );
        assert_eq!(input, &[]);
    }

    #[test]
    fn test_mixin_rule() {
        let input = ".foo(bar, baz) { }";
        let tokens = tokenize(input).unwrap();
        let mut input = &tokens[..];
        let result = item_mixin_rule(&mut input);
        assert_eq!(
            result,
            Ok(Item::MixinRule {
                name: "foo".into(),
                arguments: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
                guard: None,
                block: vec![TokenTree::Token(Token::Whitespace),],
            })
        );
        assert_eq!(input, &[]);

        let input = ".foo(bar, baz) when (true) { }";
        let tokens = tokenize(input).unwrap();
        let mut input = &tokens[..];
        let result = item_mixin_rule(&mut input);
        assert_eq!(
            result,
            Ok(Item::MixinRule {
                name: "foo".into(),
                arguments: vec![
                    TokenTree::Token(Token::Ident("bar".into())),
                    TokenTree::Token(Token::Symbol(',')),
                    TokenTree::Token(Token::Whitespace),
                    TokenTree::Token(Token::Ident("baz".into())),
                ],
                guard: Some(vec![TokenTree::Token(Token::Ident("true".into()))]),
                block: vec![TokenTree::Token(Token::Whitespace),],
            })
        );
        assert_eq!(input, &[]);
    }

    #[test]
    fn test_qualified_rule() {
        let input = "foo { }";
        let tokens = tokenize(input).unwrap();
        let mut input = &tokens[..];
        let result = item_qualified_rule(&mut input);
        assert_eq!(
            result,
            Ok(Item::QualifiedRule {
                selectors: vec![TokenTree::Token(Token::Ident("foo".into()))],
                guard: None,
                block: vec![TokenTree::Token(Token::Whitespace),],
            })
        );
        assert_eq!(input, &[]);
    }
}
