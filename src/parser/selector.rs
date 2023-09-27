use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case};
use nom::combinator::{cut, into, value};
use nom::IResult;
use nom::multi::{fold_many0, separated_list1};
use nom::sequence::{pair, preceded, terminated};

use crate::ast::{Combinator, Selector, SelectorGroup, SimpleSelector, SimpleSelectorSequence};
use crate::lexer::{ident, name, parse, symbol, token};
use crate::lexer::junk::junk1;

pub fn selector_group(input: &str) -> IResult<&str, SelectorGroup> {
    into(separated_list1(symbol(","), selector))(input)
}

pub fn selector(input: &str) -> IResult<&str, Selector> {
    let (input, first) = simple_selector_sequence(input)?;
    token(into(fold_many0(
        pair(combinator, simple_selector_sequence),
        move || (vec![first.clone()], vec![]),
        |mut acc, (c, s)| {
            acc.0.push(s);
            acc.1.push(c);
            acc
        },
    )))(input)
}

/// Consume a combinator (e.g. `>`, `+`, `~`, ` `)
pub fn combinator(input: &str) -> IResult<&str, Combinator> {
    alt((
        value(Combinator::Child, parse(symbol(">"))),
        value(Combinator::NextSibling, parse(symbol("+"))),
        value(Combinator::SubsequentSibling, parse(symbol("~"))),
        value(Combinator::Descendant, junk1),
    ))(input)
}

pub fn simple_selector_sequence(input: &str) -> IResult<&str, SimpleSelectorSequence> {
    // TODO: Parse LESS parent selector

    // Type/Universal selector can only be the first selector
    let (input, first) = alt((
        type_selector,
        universal_selector,
        id_selector,
        class_selector,
        negation_selector,
        pseudo_element_selector,
        pseudo_class_selector,
    ))(input)?;

    into(fold_many0(
        alt((
            id_selector,
            class_selector,
            negation_selector,
            pseudo_element_selector,
            pseudo_class_selector,
        )),
        move || vec![first.clone()],
        |mut acc: Vec<_>, item| {
            acc.push(item);
            acc
        },
    ))(input)
}

fn type_selector(input: &str) -> IResult<&str, SimpleSelector> {
    let (input, name) = ident(input)?;
    Ok((input, SimpleSelector::Type(name)))
}

fn universal_selector(input: &str) -> IResult<&str, SimpleSelector> {
    let (input, _) = tag("*")(input)?;
    Ok((input, SimpleSelector::Universal))
}

pub fn id_selector(input: &str) -> IResult<&str, SimpleSelector> {
    let (input, name) = preceded(tag("#"), name)(input)?;
    Ok((input, SimpleSelector::Id(name)))
}

pub fn class_selector(input: &str) -> IResult<&str, SimpleSelector> {
    let (input, name) = preceded(tag("."), ident)(input)?;
    Ok((input, SimpleSelector::Class(name)))
}

fn pseudo_class_selector(input: &str) -> IResult<&str, SimpleSelector> {
    let (input, name) = preceded(tag(":"), ident)(input)?;
    Ok((input, SimpleSelector::PseudoClass(name)))
}

fn pseudo_element_selector(input: &str) -> IResult<&str, SimpleSelector> {
    let (input, name) = preceded(tag("::"), ident)(input)?;
    Ok((input, SimpleSelector::PseudoElement(name)))
}

fn negation_selector(input: &str) -> IResult<&str, SimpleSelector> {
    let (input, arg) = preceded(
        token(tag_no_case(":not(")),
        cut(terminated(
            token(alt((
                type_selector,
                universal_selector,
                id_selector,
                class_selector,
                pseudo_element_selector,
                pseudo_class_selector,
            ))),
            tag(")"),
        )),
    )(input)?;
    Ok((input, SimpleSelector::Negation(arg.into())))
}

#[cfg(test)]
mod tests {
    use nom::Err::Failure;
    use nom::error::{ErrorKind, ParseError};

    use crate::ast::{Combinator, Selector, SelectorGroup, SimpleSelectorSequence};
    use crate::ast::SimpleSelector::*;
    use crate::parser::selector::selector_group;

    use super::simple_selector_sequence;

    #[test]
    fn test_simple_selector() {
        let cases = vec![
            ("body", Ok(("", vec![Type("body".into())].into()))),
            ("*", Ok(("", vec![Universal].into()))),
            ("#id", Ok(("", vec![Id("id".into())].into()))),
            (".class", Ok(("", vec![Class("class".into())].into()))),
            (
                ":pseudo-class",
                Ok(("", vec![PseudoClass("pseudo-class".into())].into())),
            ),
            (
                "::pseudo-element",
                Ok(("", vec![PseudoElement("pseudo-element".into())].into())),
            ),
            // Negated selectors
            (
                ":not(body)",
                Ok(("", vec![Negation(Box::from(Type("body".into())))].into())),
            ),
            (":not(*)", Ok(("", vec![Negation(Box::from(Universal))].into()))),
            (
                ":not(#id)",
                Ok(("", vec![Negation(Box::from(Id("id".into())))].into())),
            ),
            (
                ":not(.class)",
                Ok(("", vec![Negation(Box::from(Class("class".into())))].into())),
            ),
            (
                ":not(:pseudo-class)",
                Ok((
                    "",
                    vec![Negation(Box::from(PseudoClass("pseudo-class".into())))].into(),
                )),
            ),
            (
                ":not(::pseudo-element)",
                Ok((
                    "",
                    vec![Negation(Box::from(PseudoElement("pseudo-element".into())))].into(),
                )),
            ),
            (
                ":not(body.class)",
                Err(Failure(ParseError::from_error_kind(".class)", ErrorKind::Tag))),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(simple_selector_sequence(input), expected);
        }
    }

    #[test]
    fn test_simple_selector_sequence() {
        let cases = vec![
            (
                "body.class",
                Ok(("", vec![Type("body".into()), Class("class".into())].into())),
            ),
            (
                "body:pseudo",
                Ok(("", vec![Type("body".into()), PseudoClass("pseudo".into())].into())),
            ),
            (
                "body:not(:pseudo)",
                Ok((
                    "",
                    vec![
                        Type("body".into()),
                        Negation(Box::from(PseudoClass("pseudo".into()))),
                    ].into(),
                )),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(simple_selector_sequence(input), expected);
        }
    }

    #[test]
    fn test_selector() {
        let input = "body.class#id:pseudo:not(.not)::pseudo-elem > test + test test~test, a";

        assert_eq!(
            selector_group(input),
            Ok((
                "",
                SelectorGroup(vec![
                    Selector(
                        vec![
                            SimpleSelectorSequence(vec![
                                Type("body".into()),
                                Class("class".into()),
                                Id("id".into()),
                                PseudoClass("pseudo".into()),
                                Negation(Class("not".into()).into()),
                                PseudoElement("pseudo-elem".into()),
                            ]),
                            SimpleSelectorSequence(vec![Type("test".into())]),
                            SimpleSelectorSequence(vec![Type("test".into())]),
                            SimpleSelectorSequence(vec![Type("test".into())]),
                            SimpleSelectorSequence(vec![Type("test".into())]),
                        ],
                        vec![
                            Combinator::Child,
                            Combinator::NextSibling,
                            Combinator::Descendant,
                            Combinator::SubsequentSibling
                        ]
                    ),
                    Selector(
                        vec![SimpleSelectorSequence(vec![Type(
                            "a".into()
                        )])],
                        vec![]
                    )
                ])
            ))
        );
    }

}
