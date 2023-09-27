use std::borrow::Cow;

use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case, take_while1};
use nom::combinator::{map, opt};
use nom::Err::Error;
use nom::error::ErrorKind;
use nom::IResult;
use nom::sequence::{pair, preceded, terminated};

use crate::lexer::helpers::{is_digit, is_name, would_start_identifier};
use crate::util::peek_pred;

pub mod junk;
mod helpers;

#[cfg(test)]
mod tests;

/// Removes junk before applying a parser `f`.
pub fn parse<'i, O, F>(f: F) -> impl Fn(&'i str) -> IResult<&'i str, O>
    where
        O: 'i,
        F: Fn(&'i str) -> IResult<&'i str, O>,
{
    move |input: &'i str| {
        preceded(junk::junk0, &f)(input)
    }
}

/// Removes junk after applying a parser `f`.
pub fn token<'i, O, F>(f: F) -> impl Fn(&'i str) -> IResult<&'i str, O>
    where
        O: 'i,
        F: Fn(&'i str) -> IResult<&'i str, O>,
{
    move |input: &'i str| {
        terminated(&f, junk::junk0)(input)
    }
}

/// Removes junk after matching the string `xs`.
pub fn symbol<'i>(xs: &'static str) -> impl Fn(&'i str) -> IResult<&'i str, &'i str>
{
    move |input: &'i str| {
        token(tag(xs))(input)
    }
}

pub fn name(input: &str) -> IResult<&str, Cow<str>> {
    map(take_while1(is_name), |s: &str| s.into())(input)
}

pub fn ident(input: &str) -> IResult<&str, Cow<str>> {
    map(preceded(
        peek_pred(would_start_identifier),
        take_while1(is_name),
    ), |s: &str| s.into())(input)
}

pub fn at_keyword(input: &str) -> IResult<&str, Cow<str>> {
    token(preceded(tag("@"), ident))(input)
}

/// Parse a numeric value (e.g. `30`, `30px`, `30%`)
pub fn numeric(input: &str) -> IResult<&str, (f32, Option<Cow<str>>)> {
    pair(
        number,
        opt(alt((
            map(tag("%"), |c: &str| c.into()),
            name,
        ))),
    )(input)
}

/// Parse a number literal.
fn number(input: &str) -> IResult<&str, f32> {
    // Sign
    let (input, s) = opt_sign(input)?;

    // Integer and fractional parts
    let (input, (i, f, d)) = alt((
        // Integer part + optional fractional part
        map(
            pair(dec_digits, opt(preceded(tag("."), dec_digits))),
            |o| match o {
                ((i, _), Some((f, d))) => (i, f, d),
                ((i, _), None) => (i, 0, 0),
            },
        ),
        // No integer part + required fractional part
        map(
            preceded(tag("."), dec_digits),
            |(f, d)| (0, f, d),
        )
    ))(input)?;

    // Exponent sign and exponent
    let (input, (t, e)) = map(
        opt(preceded(tag_no_case("e"), pair(opt_sign, dec_digits))),
        |o| match o {
            Some((t, (e, _))) => (t, e),
            None => (1, 0),
        },
    )(input)?;

    Ok((
        input,
        // See https://www.w3.org/TR/css-syntax-3/#convert-string-to-number
        s as f32 * (i as f32 + f as f32 * 10f32.powi(-(d as i32))) * 10f32.powi(t * e as i32)
    ))
}

/// Parse an optional sign.
/// Returns -1 for '-', +1 for '+', and +1 otherwise.
fn opt_sign(input: &str) -> IResult<&str, i32> {
    map(
        opt(alt((tag("+"), tag("-")))),
        |s| match s {
            Some("-") => -1,
            _ => 1,
        },
    )(input)
}

/// Parses a string of decimal digits.
/// Returns the digits as an unsigned integer and the number of digits.
fn dec_digits(input: &str) -> IResult<&str, (u32, usize)> {
    map(
        take_while1(is_digit),
        |digits: &str| (digits.parse().unwrap(), digits.len()),
    )(input)
}
