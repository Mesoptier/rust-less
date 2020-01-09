use std::borrow::Cow;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{char, multispace0, multispace1};
use nom::combinator::{map, value, opt};
use nom::IResult;
use nom::multi::{fold_many0, fold_many1, many0, many_till};
use nom::sequence::{delimited, preceded, terminated};

use crate::ast::*;
use crate::parser::helpers::*;
use crate::parser::value::*;

mod helpers;
mod value;
mod string;

fn junk1(input: &str) -> IResult<&str, &str> {
    multispace1(input)
}

fn junk0(input: &str) -> IResult<&str, &str> {
    multispace0(input)
}

/// Ignore junk (whitespace / comments) surrounding the given parser
fn ignore_junk<'i, O, F>(f: F) -> impl Fn(&'i str) -> IResult<&'i str, O>
    where
        F: Fn(&'i str) -> IResult<&'i str, O>,
        O: 'i,
{
    move |input: &str| {
        delimited(junk0, &f, junk0)(input)
    }
}

pub fn parse_stylesheet(input: &str) -> IResult<&str, Stylesheet> {
    map(parse_list_of_items, |items| Stylesheet { items })(input)
}

fn parse_list_of_items(input: &str) -> IResult<&str, Vec<Item>> {
    many0(ignore_junk(parse_item))(input)
}

fn parse_item(input: &str) -> IResult<&str, Item> {
    map(alt((
        variable_declaration,
        declaration,
    )), |kind| Item { kind })(input)
}

fn parse_at_rule(input: &str) -> IResult<&str, ItemKind> {
    variable_declaration(input)
}

/// Parse a variable declaration (e.g. `@primary: blue;`)
fn variable_declaration(input: &str) -> IResult<&str, ItemKind> {
    let (input, name) = ignore_junk(tok_at_keyword)(input)?;
    let (input, _) = char(':')(input)?;
    let (input, value) = terminated(ignore_junk(variable_declaration_value), tag(";"))(input)?;
    Ok((input, ItemKind::VariableDeclaration { name, value }))
}

/// Parse a property declaration (e.g. `color: blue !important;`)
fn declaration(input: &str) -> IResult<&str, ItemKind> {
    let (input, name) = ignore_junk(name)(input)?;
    let (input, _) = char(':')(input)?;
    let (input, value) = ignore_junk(declaration_value)(input)?;
    let (input, important) = ignore_junk(important)(input)?;
    let (input, _) = char(';')(input)?;
    Ok((input, ItemKind::Declaration { name, value, important }))
}

/// Parse an !important token
fn important(input: &str) -> IResult<&str, bool> {
    map(
        opt(tag("!important")),
        |o| match o {
            None => false,
            Some(_) => true,
        },
    )(input)
}

fn parse_qualified_rule(input: &str) -> IResult<&str, ItemKind> {
    value(ItemKind::QualifiedRule, tag("test"))(input)
}

/// Parse a at-keyword token
/// https://www.w3.org/TR/css-syntax-3/#consume-token
fn tok_at_keyword(input: &str) -> IResult<&str, Cow<str>> {
    preceded(char('@'), name)(input)
}

/// Parse a name token
/// https://www.w3.org/TR/css-syntax-3/#consume-name
fn name(input: &str) -> IResult<&str, Cow<str>> {
    // TODO: Parse escaped code points
    map(take_while1(is_name), |name: &str| name.into())(input)
}

#[cfg(test)]
mod tests {
    use crate::ast::Value::{CommaList, Ident, SpaceList};

    use super::*;

    #[test]
    fn test_variable_declaration() {
        let cases = vec![
            ("@color: blue test;", Ok(("", ItemKind::VariableDeclaration {
                name: "color".into(),
                value: SpaceList(vec![
                    Ident("blue".into()),
                    Ident("test".into()),
                ]),
            }))),
        ];

        for (input, expected) in cases {
            assert_eq!(variable_declaration(input), expected);
        }
    }

    #[test]
    fn test_declaration() {
        let cases = vec![
            ("color: blue;", Ok(("", ItemKind::Declaration {
                name: "color".into(),
                value: Ident("blue".into()),
                important: false,
            }))),
            ("color: blue !important;", Ok(("", ItemKind::Declaration {
                name: "color".into(),
                value: Ident("blue".into()),
                important: true,
            }))),
        ];

        for (input, expected) in cases {
            assert_eq!(declaration(input), expected);
        }
    }

    #[test]
    fn test_name() {
        let cases: Vec<(&str, IResult<&str, Cow<str>>)> = vec![
            ("a", Ok(("", "a".into()))),
            ("name", Ok(("", "name".into()))),
            ("with-hyphen", Ok(("", "with-hyphen".into()))),
        ];

        for (input, expected) in cases {
            assert_eq!(name(input), expected);
        }
    }
}