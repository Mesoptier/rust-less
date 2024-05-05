use std::marker::PhantomData;

use crate::lexer::{Spanned, TokenTree};

#[derive(Clone, Debug, PartialEq)]
pub struct Stylesheet<'tokens, 'src> {
    pub items: Vec<Spanned<Item<'tokens, 'src>>>,
}

/// Items:
///  - [`AtRule`]
///      - [`MediaAtRule`] (e.g. `@media screen and (min-width: 480px) { color: blue; }`)
///      - [`KeyframesAtRule`] (e.g. `@keyframes fade { 0% { opacity: 0; } 100% { opacity: 1; } }`)
///      - etc.
///  - [`QualifiedRule`]
///      - [`StyleRule`] (e.g. `.main > a { color: blue; }`)
///      - [`MixinRule`] (e.g. `.mixin(@color) { color: @color; }`)
///      - [`KeyframeRule`] (e.g. `0% { opacity: 0; }`)
///      - etc.
///  - [`Declaration`]
///      - [`DeclarationName::Ident`] (e.g. `color: blue;`)
///      - [`DeclarationName::InterpolatedIdent`] (e.g. `@{property}: blue;` or `border-@{side}-color: blue;`)
///      - [`DeclarationName::Variable`] (e.g. `@color: blue;` or `@detached-ruleset: { color: blue; };`)
///  - [`Call`]
///      - [`MixinCall`] (e.g. `.mixin(blue);`)
///      - [`VariableCall`] (e.g. `@detached-ruleset();`)

#[derive(Clone, Debug, PartialEq)]
pub enum Item<'tokens, 'src> {
    AtRule(AtRule<'tokens, 'src>),
    QualifiedRule(QualifiedRule<'tokens, 'src>),
    Declaration(Declaration<'tokens, 'src>),
    Call(Call<'tokens, 'src>),
}

// AT-RULES

#[derive(Clone, Debug, PartialEq)]
pub enum AtRule<'tokens, 'src> {
    Generic(GenericAtRule<'tokens, 'src>),
    // TODO: Media, Keyframes, etc.
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenericAtRule<'tokens, 'src> {
    pub name: &'src str,
    // TODO: Support LESS interpolation in prelude.
    pub prelude: &'tokens [Spanned<TokenTree<'src>>],
    pub block: Option<Vec<Spanned<Item<'tokens, 'src>>>>,
}

// QUALIFIED RULES

#[derive(Clone, Debug, PartialEq)]
pub enum QualifiedRule<'tokens, 'src> {
    Generic(GenericRule<'tokens, 'src>),
    Style(StyleRule<'tokens, 'src>),
    Mixin(MixinRule<'tokens, 'src>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenericRule<'tokens, 'src> {
    // TODO: Support LESS interpolation in prelude? We certainly don't want to do so for MixinRules.
    pub prelude: &'tokens [Spanned<TokenTree<'src>>],
    pub block: Vec<Spanned<Item<'tokens, 'src>>>,
}

// TODO: Placeholder type
type Guard<'tokens, 'src> = &'tokens [Spanned<TokenTree<'src>>];

#[derive(Clone, Debug, PartialEq)]
pub struct StyleRule<'tokens, 'src> {
    pub selectors: &'tokens [Spanned<TokenTree<'src>>],
    pub guard: Option<Guard<'tokens, 'src>>,
    pub block: Vec<Spanned<Item<'tokens, 'src>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MixinRule<'tokens, 'src> {
    pub name: &'src str,
    pub arguments: &'tokens [Spanned<TokenTree<'src>>],
    pub guard: Option<Guard<'tokens, 'src>>,
    pub block: Vec<Spanned<Item<'tokens, 'src>>>,
}

// DECLARATIONS

#[derive(Clone, Debug, PartialEq)]
pub struct Declaration<'tokens, 'src> {
    pub name: DeclarationName<'tokens, 'src>,
    pub value: &'tokens [Spanned<TokenTree<'src>>],
    pub important: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeclarationName<'tokens, 'src> {
    Ident(&'src str),
    InterpolatedIdent(&'tokens [Spanned<TokenTree<'src>>]),
    Variable(&'src str),
}

// CALLS

#[derive(Clone, Debug, PartialEq)]
pub enum Call<'tokens, 'src> {
    Mixin(MixinCall<'tokens, 'src>),
    Variable(VariableCall<'tokens, 'src>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MixinCall<'tokens, 'src> {
    pub selectors: &'tokens [Spanned<TokenTree<'src>>],
    pub arguments: &'tokens [Spanned<TokenTree<'src>>],
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableCall<'tokens, 'src> {
    pub name: &'src str,
    // TODO: Support lookups.
    _lookups: PhantomData<&'tokens ()>,
}
