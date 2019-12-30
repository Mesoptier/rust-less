use nom::{
    IResult,
    Err,
    error::ErrorKind,
    branch::alt,
    sequence::{tuple, pair, preceded, delimited, terminated},
    combinator::{opt, map, verify, recognize},
    character::complete::{alpha1, space0, char},
    bytes::complete::tag,
    multi::many1,
};
use crate::grammar::{Selector, SelectorChild, TypeSelector, ClassSelector, Combinator};

pub fn selector(input: &str) -> IResult<&str, Selector> {
    many1(selector_child)(input).map(|(i, o)| (i, Selector { children: o }))
}

fn selector_child(input: &str) -> IResult<&str, SelectorChild> {
    alt((
        map(class_selector, |o| SelectorChild::ClassSelector(o)),
        map(type_selector, |o| SelectorChild::TypeSelector(o)),
        map(combinator, |o| SelectorChild::Combinator(o)),
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

/// To match a combinator we either look for one of the combinator symbols (>, +, ~) possibly
/// surrounded with whitespace or we look for some whitespace.
fn combinator(input: &str) -> IResult<&str, Combinator> {
    map(verify(
        tuple((
            space0,
            opt(alt((
                map(char('>'), |o| Combinator::Child),
                map(char('+'), |o| Combinator::AdjacentSibling),
                map(char('~'), |o| Combinator::GeneralSibling),
            ))),
            space0,
        )),
        |(ws, o, _): &(&str, Option<Combinator>, _)| o.is_some() || !ws.is_empty(),
    ), |(_, o, _)| {
        if let Some(res) = o {
            return res;
        } else {
            return Combinator::Descendant;
        }
    })(input)
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

    #[test_case(">" => Ok(("", Combinator::Child)); "child combinator")]
    #[test_case(" > " => Ok(("", Combinator::Child)); "child combinator + spaces")]
    #[test_case("+" => Ok(("", Combinator::AdjacentSibling)); "adjacent sibling combinator")]
    #[test_case(" + " => Ok(("", Combinator::AdjacentSibling)); "adjacent sibling combinator + spaces")]
    #[test_case("~" => Ok(("", Combinator::GeneralSibling)); "general sibling combinator")]
    #[test_case(" ~ " => Ok(("", Combinator::GeneralSibling)); "general sibling combinator + spaces")]
    #[test_case(" " => Ok(("", Combinator::Descendant)); "descendant combinator")]
    #[test_case("" => Err(Err::Error(("", ErrorKind::Verify))))]
    fn test_combinator(input: &str) -> IResult<&str, Combinator> {
        combinator(input)
    }
}