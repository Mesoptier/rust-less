use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case};
use nom::combinator::{cut, opt, value};
use nom::IResult;
use nom::multi::{fold_many0, many0, many1, separated_nonempty_list};
use nom::sequence::{delimited, pair, preceded, terminated};

use crate::ast::{Combinator, Selector, SelectorGroup, SimpleSelector, SimpleSelectorSequence};
use crate::lexer::{ident, name, parse, symbol, token};
use crate::lexer::junk::{junk0, junk1};

pub fn selector_group(input: &str) -> IResult<&str, SelectorGroup> {
    separated_nonempty_list(symbol(","), selector)(input)
}

pub fn selector(input: &str) -> IResult<&str, Selector> {
    let (input, first) = simple_selector_sequence(input)?;
    token(fold_many0(
        pair(combinator, simple_selector_sequence),
        (vec![first], vec![]),
        |mut acc, (c, s)| {
            acc.0.push(s);
            acc.1.push(c);
            acc
        },
    ))(input)
}

/// Consume a combinator (e.g. `>`, `+`, `~`, ` `)
pub fn combinator(input: &str) -> IResult<&str, Combinator> {
    alt((
        value(Combinator::Child, parse(symbol(">"))),
        value(Combinator::NextSibling, parse(symbol("+"))),
        value(Combinator::SubsequentSibling, parse(symbol("~"))),
        value(Combinator::Descendant, junk1)
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

    fold_many0(
        alt((
            id_selector,
            class_selector,
            negation_selector,
            pseudo_element_selector,
            pseudo_class_selector,
        )),
        vec![first],
        |mut acc: Vec<_>, item| {
            acc.push(item);
            acc
        },
    )(input)
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
    let (input, arg) =
        preceded(
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
    use crate::ast::{SimpleSelectorSequence, SimpleSelector};

    use super::simple_selector_sequence;

    #[test]
    fn test_simple_selector() {
        let cases = vec![
            ("body", Ok(("", vec![SimpleSelector::Type("body".into())]))),
            ("*", Ok(("", vec![SimpleSelector::Universal]))),
            ("#id", Ok(("", vec![SimpleSelector::Id("id".into())]))),
            (".class", Ok(("", vec![SimpleSelector::Class("class".into())]))),
            (":pseudo-class", Ok(("", vec![SimpleSelector::PseudoClass("pseudo-class".into())]))),
            ("::pseudo-element", Ok(("", vec![SimpleSelector::PseudoElement("pseudo-element".into())]))),

            // Negated selectors
            (":not(body)", Ok(("", vec![SimpleSelector::Negation(Box::from(SimpleSelector::Type("body".into())))]))),
            (":not(*)", Ok(("", vec![SimpleSelector::Negation(Box::from(SimpleSelector::Universal))]))),
            (":not(#id)", Ok(("", vec![SimpleSelector::Negation(Box::from(SimpleSelector::Id("id".into())))]))),
            (":not(.class)", Ok(("", vec![SimpleSelector::Negation(Box::from(SimpleSelector::Class("class".into())))]))),
            (":not(:pseudo-class)", Ok(("", vec![SimpleSelector::Negation(Box::from(SimpleSelector::PseudoClass("pseudo-class".into())))]))),
            (":not(::pseudo-element)", Ok(("", vec![SimpleSelector::Negation(Box::from(SimpleSelector::PseudoElement("pseudo-element".into())))]))),
        ];

        for (input, expected) in cases {
            assert_eq!(simple_selector_sequence(input), expected);
        }
    }
}