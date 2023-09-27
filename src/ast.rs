use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub struct Stylesheet<'i> {
    pub items: Vec<Item<'i>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item<'i> {
    /// A CSS at-rule (e.g. `@media ... { ... }`)
    AtRule,
    /// A CSS qualified rule (e.g. `body > a { ... }`)
    QualifiedRule {
        selector_group: SelectorGroup<'i>,
        block: Vec<Item<'i>>,
    },
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
    VariableCall {
        name: Cow<'i, str>,
    },
    /// A LESS mixin declaration (e.g. `.mixin(@arg) { ... }`)
    MixinDeclaration {
        selector: SimpleSelector<'i>,
        block: Vec<Item<'i>>,
    },
    /// A LESS mixin call (e.g. `.mixin(@arg: 'blue');`)
    MixinCall {
        selector: Vec<SimpleSelector<'i>>,
    },
}


//
// Values
//

#[derive(Clone, Debug, PartialEq)]
pub enum InterpolatedValue<'i> {
    Variable(Cow<'i, str>),
    Property(Cow<'i, str>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'i> {
    /// A semicolon-separated list of values
    SemicolonList(Vec<Value<'i>>),
    /// A comma-separated list of values
    CommaList(Vec<Value<'i>>),
    /// A space-separated list of values
    SpaceList(Vec<Value<'i>>),

    /// A detached ruleset (e.g. `{ color: blue; }`)
    DetachedRuleset(Vec<Item<'i>>),

    /// A binary operation (e.g. `2px + @spacing`)
    Operation(Operation, Box<Value<'i>>, Box<Value<'i>>),

    /// A variable reference (e.g. `@primary`)
    Variable(Cow<'i, str>),
    /// A variable lookup (e.g. `@colors[primary]`)
    VariableLookup(Cow<'i, str>, Vec<Lookup<'i>>),
    /// A property reference (e.g. `$color`)
    Property(Cow<'i, str>),
    /// An ident (e.g. `border-collapse`)
    Ident(Cow<'i, str>),
    /// A number (e.g. `20`, `20.5e-2`, `20%`, `20px`)
    Numeric(f32, Option<Cow<'i, str>>),
    /// A function call (e.g. `rgba(0, 0, 0, 0.5)`)
    FunctionCall(Cow<'i, str>, Box<Value<'i>>),
    /// A quoted string (e.g. `"test"`)
    QuotedString(Cow<'i, str>),
    /// An interpolated string (e.g. `"color is @{color}"`, `"color is ${color}"`)
    InterpolatedString(Vec<Cow<'i, str>>, Vec<InterpolatedValue<'i>>),
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
    /// An interpolated string (e.g. `"color is @{color}"`, `"color is ${color}"`)
    InterpolatedString(Vec<Cow<'i, str>>, Vec<InterpolatedValue<'i>>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}


//
// Selectors
//

pub type SelectorGroup<'i> = Vec<Selector<'i>>;
pub type Selector<'i> = (Vec<SimpleSelectorSequence<'i>>, Vec<Combinator>);
pub type SimpleSelectorSequence<'i> = Vec<SimpleSelector<'i>>;

#[derive(Clone, Debug, PartialEq)]
pub enum Combinator {
    Descendant,
    Child,
    NextSibling,
    SubsequentSibling,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SimpleSelector<'i> {
    Type(Cow<'i, str>),
    Universal,
    Id(Cow<'i, str>),
    Class(Cow<'i, str>),
    Attribute(Cow<'i, str>),
    // TODO: Support functional pseudo-classes/pseudo-elements
    PseudoClass(Cow<'i, str>),
    PseudoElement(Cow<'i, str>),
    Negation(Box<SimpleSelector<'i>>),
}
