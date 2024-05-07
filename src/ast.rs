use std::marker::PhantomData;

use crate::lexer::{Spanned, TokenTree};

#[derive(Clone, Debug, PartialEq)]
pub struct Stylesheet<'tokens, 'src> {
    pub items: ListOfItems<'tokens, 'src>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListOfItems<'tokens, 'src>(pub Vec<Spanned<Item<'tokens, 'src>>>);

#[derive(Clone, Debug, PartialEq)]
pub struct ListOfComponentValues<'tokens, 'src>(pub &'tokens [Spanned<TokenTree<'src>>]);

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
///      - [`FunctionCall`] (e.g. `each(red blue green, {});`)

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
    pub prelude: ListOfComponentValues<'tokens, 'src>,
    pub block: Option<ListOfItems<'tokens, 'src>>,
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
    pub prelude: ListOfComponentValues<'tokens, 'src>,
    pub block: ListOfItems<'tokens, 'src>,
}

// TODO: Placeholder type
type Guard<'tokens, 'src> = ListOfComponentValues<'tokens, 'src>;

#[derive(Clone, Debug, PartialEq)]
pub struct StyleRule<'tokens, 'src> {
    pub selectors: ListOfComponentValues<'tokens, 'src>,
    pub guard: Option<Guard<'tokens, 'src>>,
    pub block: ListOfItems<'tokens, 'src>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MixinRule<'tokens, 'src> {
    pub name: &'src str,
    pub arguments: ListOfComponentValues<'tokens, 'src>,
    pub guard: Option<Guard<'tokens, 'src>>,
    pub block: ListOfItems<'tokens, 'src>,
}

// DECLARATIONS

#[derive(Clone, Debug, PartialEq)]
pub struct Declaration<'tokens, 'src> {
    pub name: DeclarationName<'tokens, 'src>,
    pub value: ListOfComponentValues<'tokens, 'src>,
    pub important: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeclarationName<'tokens, 'src> {
    Ident(&'src str),
    InterpolatedIdent(ListOfComponentValues<'tokens, 'src>),
    Variable(&'src str),
}

// CALLS

#[derive(Clone, Debug, PartialEq)]
pub enum Call<'tokens, 'src> {
    Mixin(MixinCall<'tokens, 'src>),
    Variable(VariableCall<'tokens, 'src>),
    Function(FunctionCall<'tokens, 'src>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MixinCall<'tokens, 'src> {
    pub selector: ListOfComponentValues<'tokens, 'src>,
    pub arguments: ListOfComponentValues<'tokens, 'src>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableCall<'tokens, 'src> {
    pub name: &'src str,
    // TODO: Support lookups.
    pub _lookups: PhantomData<&'tokens ()>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionCall<'tokens, 'src> {
    pub name: &'src str,
    pub arguments: ListOfComponentValues<'tokens, 'src>,
}
