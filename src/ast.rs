use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub struct Stylesheet<'i> {
    pub items: Vec<Item<'i>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block<'i> {
    pub stmts: Vec<Item<'i>>,
}

/// A statement.
///
/// Can be anything that is valid at the top level of a stylesheet or a qualified block.
#[derive(Clone, Debug, PartialEq)]
pub struct Item<'i> {
    pub kind: ItemKind<'i>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind<'i> {
    /// A CSS at-rule (e.g. `@media ... { ... }`)
    AtRule(Box<AtRule<'i>>),
    /// A CSS qualified rule (e.g. `body > a { ... }`)
    QualifiedRule,
    /// A CSS property declaration (e.g. `color: blue;`)
    Declaration {
        name: Cow<'i, str>,
        value: Value<'i>,
        important: bool,
    },
    /// A LESS variable declaration (e.g. `@color: blue;`)
    VariableDeclaration {
        name: Cow<'i, str>,
        value: Value<'i>,
    },
    /// A LESS variable call (e.g. `@ruleset();`)
    VariableCall,
    /// A LESS mixin declaration (e.g. `.mixin(@arg) { ... }`)
    MixinDeclaration,
    /// A LESS mixin call (e.g. `.mixin(@arg: 'blue');`)
    MixinCall,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'i> {
    /// A semicolon-separated list of values
    SemicolonList(Vec<Value<'i>>),
    /// A comma-separated list of values
    CommaList(Vec<Value<'i>>),
    /// A space-separated list of values
    SpaceList(Vec<Value<'i>>),

    /// A variable reference (e.g. `@primary`)
    Variable(Cow<'i, str>),
    /// A variable lookup (e.g. `@colors[primary]`)
    VariableLookup(Cow<'i, str>, Vec<Lookup<'i>>),
    /// A property reference (e.g. `$color`)
    Property(Cow<'i, str>),
    /// A detached ruleset (e.g. `{ color: blue; }`)
    DetachedRuleset,
    /// An ident (e.g. `border-collapse`)
    Ident(Cow<'i, str>),
    /// A number (e.g. `20`, `20.5e-2`, `20%`, `20px`)
    Numeric(f32, Option<Cow<'i, str>>),
    /// A function call (e.g. `rgba(0, 0, 0, 0.5)`)
    FunctionCall(Cow<'i, str>, Box<Value<'i>>),
    /// A quoted string (e.g. `"test"`)
    QuotedString(Cow<'i, str>),
    /// An interpolated string (e.g. `"color is @{color}"`)
    InterpolatedString(Vec<Cow<'i, str>>, Vec<Value<'i>>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Lookup<'i> {
    /// Lookup last declaration (e.g. `@config[]`)
    Last,
    /// Lookup property declaration by ident (e.g. `@config[property]`)
    Ident(Cow<'i, str>),
    /// Lookup variable declaration by ident (e.g. `@config[@variable]`)
    Variable(Cow<'i, str>),
    /// Lookup property declaration by ident (e.g. `@config[$property]`)
    Property(Cow<'i, str>),
    /// Lookup variable declaration by variable (e.g. `@config[@@variable]`)
    VariableVariable(Cow<'i, str>),
    /// Lookup property declaration by variable (e.g. `@config[$@variable]`)
    VariableProperty(Cow<'i, str>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AtRule<'i> {
    pub kind: AtRuleKind<'i>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AtRuleKind<'i> {
    Media {
        // TODO: prelude
        block: Vec<Item<'i>>,
    },
    Import {
        options: Vec<ImportOption>,
        filename: String,
    },
    Plugin {
        name: String,
    },
    Other {
        name: String,
        // TODO: What kind of items should the prelude/block consist of? Maybe just Tokens?
        prelude: Vec<String>,
        block: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportOption {
    Reference,
    Inline,
    LESS,
    CSS,
    Once,
    Multiple,
    Optional,
}