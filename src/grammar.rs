use alloc::vec::Vec;

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

pub enum Combinator {
    Descendant,         // ' '
    Child,              // '>'
    AdjacentSibling,    // '+'
    GeneralSibling,     // '~'
}

pub struct Element<'i> {
    combinator: Combinator,
    name: &'i str,
}

pub struct Selector<'i> {
    elements: Vec<Element<'i>>,
}