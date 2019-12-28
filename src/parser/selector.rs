use nom::IResult;
use nom::branch::alt;
use nom::sequence::{tuple, pair, preceded};
use nom::combinator::{opt, map};
use nom::character::complete::alpha1;
use nom::bytes::complete::tag;
use nom::multi::many1;
use crate::grammar::{Selector, SelectorChild, TypeSelector, ClassSelector};

pub fn selector(input: &str) -> IResult<&str, Selector> {
    many1(selector_child)(input).map(|(i, o)| (i, Selector { children: o }))
}

fn selector_child(input: &str) -> IResult<&str, SelectorChild> {
    alt((
        map(class_selector, |o| SelectorChild::ClassSelector(o)),
        map(type_selector, |o| SelectorChild::TypeSelector(o)),
    ))(input)
}

fn class_selector(input: &str) -> IResult<&str, ClassSelector> {
    map(preceded(tag("."), identifier), |o| ClassSelector { name: o })(input)
}

fn type_selector(input: &str) -> IResult<&str, TypeSelector> {
    identifier_or_asterisk(input).map(|(i, o)| (i, TypeSelector { name: o }))
}

fn identifier_or_asterisk(input: &str) -> IResult<&str, &str> {
    alt((
        tag("*"),
        identifier,
    ))(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    // TODO: Make this conform to CSS spec
    alpha1(input)
}

#[cfg(test)]
mod tests {
    use test_case::test_case;
    use nom::IResult;

    use super::*;

    #[test_case(
    ".a" => Ok(("", Selector{ children: vec ! [SelectorChild::ClassSelector(ClassSelector{ name: "a" })] }));
    "class selector"
    )]
    #[test_case(
    "a" => Ok(("", Selector{ children: vec ! [SelectorChild::TypeSelector(TypeSelector{ name: "a" })] }));
    "type selector"
    )]
    fn test_selector(input: &str) -> IResult<&str, Selector> {
        selector(input)
    }
}