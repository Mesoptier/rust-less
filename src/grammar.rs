use alloc::vec::Vec;

/// # CSS grammar
/// Based on https://github.com/csstree/csstree/blob/master/docs/ast.md

pub struct StyleSheet<'i> {
    children: Vec<_StyleSheetChild<'i>>,
}
enum _StyleSheetChild<'i> {
    Rule(Rule<'i>),
    Declaration(Declaration<'i>),
}

/// Rule
/// e.g. `.selector { <block> }`
pub struct Rule<'i> {
    prelude: _RulePrelude<'i>,
    block: Block<'i>,
}
enum _RulePrelude<'i> {
    SelectorList(SelectorList<'i>),
    Raw(Raw<'i>),
}

pub struct Block<'i> {
    children: Vec<_BlockChild<'i>>,
}
enum _BlockChild<'i> {
    Declaration(Declaration<'i>),
}

/// List of selectors
/// e.g. `a, .b > c, ...`
pub struct SelectorList<'i> {
    pub children: Vec<Selector<'i>>,
}

/// Selector
/// e.g. `body > .class`
#[derive(Debug, PartialEq)]
pub struct Selector<'i> {
    pub children: Vec<SelectorChild<'i>>,
}

#[derive(Debug, PartialEq)]
pub enum SelectorChild<'i> {
    AttributeSelector(AttributeSelector<'i>),
    ClassSelector(ClassSelector<'i>),
    IdSelector(IdSelector<'i>),
    PseudoClassSelector(PseudoClassSelector<'i>),
    PseudoElementSelector(PseudoElementSelector<'i>),
    TypeSelector(TypeSelector<'i>),
    Combinator(Combinator),
}

#[derive(Debug, PartialEq)]
pub struct AttributeSelector<'i> {
    // TODO
    pub raw: Raw<'i>,
}

#[derive(Debug, PartialEq)]
pub struct ClassSelector<'i> {
    pub name: &'i str,
}

#[derive(Debug, PartialEq)]
pub struct IdSelector<'i> {
    pub name: &'i str,
}

#[derive(Debug, PartialEq)]
pub struct PseudoClassSelector<'i> {
    pub name: &'i str,
    pub children: Option<Vec<Raw<'i>>>,
}

#[derive(Debug, PartialEq)]
pub struct PseudoElementSelector<'i> {
    pub name: &'i str,
    pub children: Option<Vec<Raw<'i>>>,
}

#[derive(Debug, PartialEq)]
pub struct TypeSelector<'i> {
    pub name: &'i str,
}

#[derive(Debug, PartialEq)]
pub enum Combinator {
    Descendant,         // ' '
    Child,              // '>'
    AdjacentSibling,    // '+'
    GeneralSibling,     // '~'
}

#[derive(Debug, PartialEq)]
pub struct Raw<'i> {
    pub value: &'i str,
}


// OLD:

pub enum Primary<'i> {
    Ruleset(Ruleset<'i>),
    Declaration(Declaration<'i>)
}

pub struct Ruleset<'i> {
    selector: Selector<'i>,
    primary: Vec<Primary<'i>>,
}

#[derive(Debug, PartialEq)]
pub enum DeclarationName {
    Variable(String), // LESS variable
    Property(String), // CSS property
}

pub struct Declaration<'i> {
    name: DeclarationName,
    value: &'i str,
}

//
// Selectors
//
