use crate::lexer::{Spanned, TokenTree};

#[derive(Clone, Debug, PartialEq)]
pub struct Stylesheet<'tokens, 'src> {
    pub items: Vec<Spanned<Item<'tokens, 'src>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item<'tokens, 'src> {
    /// Regular CSS at-rule.
    AtRule {
        name: &'src str,
        // TODO: Support LESS interpolation in prelude.
        prelude: &'tokens [Spanned<TokenTree<'src>>],
        block: Option<Vec<Spanned<Item<'tokens, 'src>>>>,
    },
    /// Regular CSS qualified rule.
    QualifiedRule {
        // TODO: Should we ever parse this into a selector list? The prelude of a qualified rule
        //  does not have to be a selector list, it can be any list of tokens (e.g. `100%` in a
        //  `@keyframes` block).
        // TODO: Support LESS interpolation in prelude.
        prelude: &'tokens [Spanned<TokenTree<'src>>],
        guard: Option<Guard<'tokens, 'src>>,
        block: Vec<Spanned<Item<'tokens, 'src>>>,
    },
    /// Regular CSS declaration.
    Declaration {
        name: DeclarationName<'tokens, 'src>,
        value: &'tokens [Spanned<TokenTree<'src>>],
        important: bool,
    },
    /// LESS mixin rule.
    MixinRule {
        name: &'src str,
        // TODO: Parse MixinRule arguments
        arguments: &'tokens [Spanned<TokenTree<'src>>],
        guard: Option<Guard<'tokens, 'src>>,
        block: Vec<Spanned<Item<'tokens, 'src>>>,
    },
    /// LESS mixin call.
    MixinCall {
        // TODO: Parse MixinCall selector
        selector: &'tokens [Spanned<TokenTree<'src>>],
        // TODO: Parse MixinCall arguments
        arguments: &'tokens [Spanned<TokenTree<'src>>],
    },
    /// LESS variable declaration.
    VariableDeclaration {
        name: &'src str,
        // TODO: Special parsing case for detached rulesets
        value: &'tokens [Spanned<TokenTree<'src>>],
    },
    /// LESS variable call.
    VariableCall { name: &'src str },
}

// TODO: Placeholder type
pub type Guard<'tokens, 'src> = &'tokens [Spanned<TokenTree<'src>>];

#[derive(Clone, Debug, PartialEq)]
pub enum DeclarationName<'tokens, 'src> {
    Literal(&'src str),
    // TODO: Parse interpolated declaration names (e.g. `border-@{side}-radius`). This should be
    //  dealt with at the lexer level (e.g. with a `InterpolatedIdent` variant in `Token`).
    Interpolated(&'tokens [Spanned<TokenTree<'src>>]),
}

// TODO: Idea for having support for both specific and generic at-rules.
// pub enum AtRule<'tokens, 'src> {
//     Media {
//         prelude: &'tokens [Spanned<TokenTree<'src>>],
//         block: Vec<Spanned<Item<'tokens, 'src>>>,
//     },
//     Keyframes {
//         // TODO: Name might contain LESS interpolations?
//         name: &'src str,
//         // TODO: Keyframes block may only contain qualified rules (e.g. `from { color: blue }` or `0% { color: blue }`).
//         block: Vec<Spanned<Item<'tokens, 'src>>>,
//     },
//     Generic {
//         name: &'src str,
//         prelude: &'tokens [Spanned<TokenTree<'src>>],
//         block: Vec<Spanned<Item<'tokens, 'src>>>,
//     },
// }
