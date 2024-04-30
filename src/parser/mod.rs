use std::borrow::Cow;
use winnow::combinator::{alt, opt, preceded, repeat, repeat_till, terminated};
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

fn block<'i>(delim: Delim) -> impl FnMut(&mut TokenStream<'_, 'i>) -> PResult<Vec<TokenTree<'i>>> {
    move |input| {
        any.verify_map(|tt| match tt {
            // TODO: Shouldn't we be cloning the tokens here? I guess winnow is already cloning somewhere?
            TokenTree::Delim(d, tokens) if d == delim => Some(tokens),
            _ => None,
        })
        .parse_next(input)
    }
}

pub fn stylesheet<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Stylesheet<'i>> {
    preceded(whitespace, repeat(0.., terminated(item, whitespace)))
        .map(|items| Stylesheet { items })
        .parse_next(input)
}

fn item<'i>(input: &mut TokenStream<'_, 'i>) -> PResult<Item<'i>> {
    alt((
        // item_at_rule,
        // item_qualified_rule,
        // item_declaration,
        // item_mixin_rule,
        // item_mixin_call,
        item_variable_declaration,
        item_variable_call,
    ))
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
    seq!(_: symbol('@'), ident, block(Delim::Paren), _: (whitespace, symbol(';')))
        .map(|(name, arguments)| Item::VariableCall { name, arguments })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

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
}
