use std::borrow::Cow;

use crate::lexer::{Spanned, TokenTree};

#[derive(Clone, Debug, PartialEq)]
pub struct Stylesheet<'tokens, 'src> {
    pub items: Vec<Spanned<Item<'tokens, 'src>>>,
}

// TODO: Many of these fields can be parsed into more specific types.
#[derive(Clone, Debug, PartialEq)]
pub enum Item<'tokens, 'src> {
    /// Regular CSS at-rule.
    AtRule {
        name: &'src str,
        prelude: &'tokens [Spanned<TokenTree<'src>>],
        block: Option<Vec<Spanned<Item<'tokens, 'src>>>>,
    },
    /// Regular CSS qualified rule.
    QualifiedRule {
        // TODO: Rename to `prelude`?
        selectors: Vec<TokenTree<'src>>,
        guard: Option<Vec<TokenTree<'src>>>,
        block: Vec<TokenTree<'src>>,
    },
    /// Regular CSS declaration.
    Declaration {
        name: Vec<TokenTree<'src>>,
        value: Vec<TokenTree<'src>>,
        important: bool,
    },
    /// LESS mixin rule.
    MixinRule {
        name: Cow<'src, str>,
        arguments: Vec<TokenTree<'src>>,
        guard: Option<Vec<TokenTree<'src>>>,
        block: Vec<TokenTree<'src>>,
    },
    /// LESS mixin call.
    MixinCall {
        selector: Vec<TokenTree<'src>>,
        arguments: Vec<TokenTree<'src>>,
    },
    /// LESS variable declaration.
    VariableDeclaration {
        name: Cow<'src, str>,
        value: Vec<TokenTree<'src>>,
    },
    /// LESS variable call.
    VariableCall {
        name: Cow<'src, str>,
        arguments: Vec<TokenTree<'src>>,
    },
}
