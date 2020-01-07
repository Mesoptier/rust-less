use std::borrow::Cow;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::combinator::map;
use nom::IResult;
use nom::multi::separated_nonempty_list;

use crate::ast::*;
use crate::parser::helpers::{is_name, is_whitespace};
use crate::parser::ignore_junk;

pub fn comma_list(input: &str) -> IResult<&str, Value> {
    map(
        separated_nonempty_list(tag(","), ignore_junk(space_list)),
        |values| Value::CommaList(values),
    )(input)
}

pub fn space_list(input: &str) -> IResult<&str, Value> {
    map(
        // TODO: Use addition/sum_expression here instead of single_value
        separated_nonempty_list(take_while1(is_whitespace), single_value),
        |values| Value::SpaceList(values),
    )(input)
}

fn single_value(input: &str) -> IResult<&str, Value> {
    simple_value(input)
}

fn simple_value(input: &str) -> IResult<&str, Value> {
    alt((
        ident,
        ident,
    ))(input)
}

fn ident(input: &str) -> IResult<&str, Value> {
    map(name, |name| Value::Ident(name))(input)
}

fn name<'i>(input: &'i str) -> IResult<&'i str, Cow<'i, str>> {
    map(take_while1(is_name), |s: &'i str| s.into())(input)
}