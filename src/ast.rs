use std::borrow::Cow;

use crate::lexer::TokenTree;

#[derive(Clone, Debug, PartialEq)]
pub struct Stylesheet<'i> {
    pub items: Vec<Item<'i>>,
}

// TODO: Many of these fields can be parsed into more specific types.
#[derive(Clone, Debug, PartialEq)]
pub enum Item<'i> {
    /// Regular CSS at-rule.
    AtRule {
        name: Cow<'i, str>,
        prelude: Vec<TokenTree<'i>>,
        block: Option<Vec<TokenTree<'i>>>,
    },
    /// Regular CSS qualified rule.
    QualifiedRule {
        selectors: Vec<TokenTree<'i>>,
        guard: Option<Vec<TokenTree<'i>>>,
        block: Vec<TokenTree<'i>>,
    },
    /// Regular CSS declaration.
    Declaration {
        name: Vec<TokenTree<'i>>,
        value: Vec<TokenTree<'i>>,
        important: bool,
    },
    /// LESS mixin rule.
    MixinRule {
        name: Cow<'i, str>,
        arguments: Vec<TokenTree<'i>>,
        guard: Option<Vec<TokenTree<'i>>>,
        block: Vec<TokenTree<'i>>,
    },
    /// LESS mixin call.
    MixinCall {
        selector: Vec<TokenTree<'i>>,
        arguments: Vec<TokenTree<'i>>,
    },
    /// LESS variable declaration.
    VariableDeclaration {
        name: Cow<'i, str>,
        value: Vec<TokenTree<'i>>,
    },
    /// LESS variable call.
    VariableCall {
        name: Cow<'i, str>,
        arguments: Vec<TokenTree<'i>>,
    },
}
