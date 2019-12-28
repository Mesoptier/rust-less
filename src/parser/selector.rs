use nom::IResult;
use nom::branch::alt;
use nom::sequence::{tuple, pair};
use nom::combinator::opt;
use nom::character::complete::alpha1;
use crate::grammar::{Selector, SelectorChild, TypeSelector};
use nom::bytes::complete::tag;
use nom::multi::many1;

pub fn selector(input: &str) -> IResult<&str, Selector> {
    many1(selector_child)(input).map(|(i, o)| (i, Selector { children: o }))
}

fn selector_child(input: &str) -> IResult<&str, SelectorChild> {
    type_selector(input).map(|(i, o)| (i, SelectorChild::TypeSelector(o)))
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

    #[test]
    fn test_type_selector() {}

    #[test_case(
    "a" => Ok(("", Selector{ children: vec ! [SelectorChild::TypeSelector(TypeSelector{ name: "a" })] }));
    "type selector"
    )]
    fn test_selector(input: &str) -> IResult<&str, Selector> {
        selector(input)
    }
}