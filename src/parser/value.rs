use std::borrow::Cow;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while1};
use nom::character::complete::char;
use nom::combinator::{map, opt, value};
use nom::IResult;
use nom::multi::{separated_nonempty_list, many1};
use nom::sequence::{pair, preceded, separated_pair};

use crate::ast::*;
use crate::parser::{ignore_junk, name};
use crate::parser::helpers::{is_digit, is_name, is_whitespace};
use crate::parser::string::string;

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
        numeric,
        // color,
        string('"'),
        string('\''),
        // unicode_descriptor,
        variable_or_lookup,
        property,
        // url,
        // function,
        // mixin_call, // includes mixin_lookup?
        ident,
    ))(input)
}

/// Parse a variable or variable lookup (e.g. `@var`, `@var[]`)
fn variable_or_lookup(input: &str) -> IResult<&str, Value> {
    let (input, name) = preceded(tag("@"), name)(input)?;

    if let Ok((input, lookups)) = many1(lookup)(input) {
        return Ok((input, Value::VariableLookup(name, lookups)));
    }

    Ok((input, Value::Variable(name)))
}

/// Parse a lookup (e.g. `[]`, `[color]`, `[$@property]`)
fn lookup(input: &str) -> IResult<&str, Lookup> {
    let (input, _) = tag("[")(input)?;
    let (input, lookup) = alt((
        map(preceded(tag("$@"), name), Lookup::VariableProperty),
        map(preceded(tag("@@"), name), Lookup::VariableVariable),
        map(preceded(tag("$"), name), Lookup::Property),
        map(preceded(tag("@"), name), Lookup::Variable),
        map(name, Lookup::Ident),
        value(Lookup::Last, tag("")),
    ))(input)?;
    let (input, _) = tag("]")(input)?;

    Ok((input, lookup))
}

/// Parse a variable (e.g. `@var`)
fn variable(input: &str) -> IResult<&str, Value> {
    map(
        preceded(tag("@"), name),
        |name| Value::Variable(name),
    )(input)
}

/// Parse a property accessor (e.g. `$color`)
fn property(input: &str) -> IResult<&str, Value> {
    map(
        preceded(tag("$"), name),
        |name| Value::Property(name),
    )(input)
}

/// Parse a numeric value (e.g. `30`, `30px`, `30%`)
fn numeric(input: &str) -> IResult<&str, Value> {
    let (input, val) = number(input)?;
    let (input, unit) = opt(alt((
        map(tag("%"), |c: &str| c.into()),
        name,
    )))(input)?;

    Ok((input, Value::Numeric(val, unit)))
}

/// Parse a number literal.
fn number(input: &str) -> IResult<&str, f32> {
    // Sign
    let (input, s) = opt_sign(input)?;

    // Integer and fractional parts
    let (input, (i, f, d)) = alt((
        // Integer part + optional fractional part
        map(
            pair(dec_digits, opt(preceded(char('.'), dec_digits))),
            |o| match o {
                ((i, _), Some((f, d))) => (i, f, d),
                ((i, _), None) => (i, 0, 0),
            },
        ),
        // No integer part + required fractional part
        map(
            preceded(char('.'), dec_digits),
            |(f, d)| (0, f, d),
        )
    ))(input)?;

    // Exponent sign and exponent
    let (input, (t, e)) = map(
        opt(preceded(alt((char('e'), char('E'))), pair(opt_sign, dec_digits))),
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
        opt(alt((char('+'), char('-')))),
        |s| match s {
            Some('-') => -1,
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

fn ident(input: &str) -> IResult<&str, Value> {
    map(name, |name| Value::Ident(name))(input)
}

#[cfg(test)]
mod tests {
    use crate::ast::{Value, Lookup};
    use crate::parser::value::{number, numeric, property, variable, lookup, variable_or_lookup};

    #[test]
    fn test_variable_or_lookup() {
        let cases = vec![
            ("@var", Ok(("", Value::Variable("var".into())))),
            ("@last[]", Ok(("", Value::VariableLookup("last".into(), vec![Lookup::Last])))),
            ("@mult[][]", Ok(("", Value::VariableLookup("mult".into(), vec![Lookup::Last, Lookup::Last])))),
        ];

        for (input, expected) in cases {
            assert_eq!(variable_or_lookup(input), expected);
        }
    }

    #[test]
    fn test_lookup() {
        let cases = vec![
            ("[]", Ok(("", Lookup::Last))),
            ("[ident]", Ok(("", Lookup::Ident("ident".into())))),
            ("[@variable]", Ok(("", Lookup::Variable("variable".into())))),
            ("[$property]", Ok(("", Lookup::Property("property".into())))),
            ("[@@variable]", Ok(("", Lookup::VariableVariable("variable".into())))),
            ("[$@property]", Ok(("", Lookup::VariableProperty("property".into())))),
        ];

        for (input, expected) in cases {
            assert_eq!(lookup(input), expected);
        }
    }

    #[test]
    fn test_variable() {
        let cases = vec![
            ("@var", Ok(("", Value::Variable("var".into())))),
        ];

        for (input, expected) in cases {
            assert_eq!(variable(input), expected);
        }
    }

    #[test]
    fn test_property() {
        let cases = vec![
            ("$color", Ok(("", Value::Property("color".into())))),
        ];

        for (input, expected) in cases {
            assert_eq!(property(input), expected);
        }
    }

    #[test]
    fn test_numeric() {
        let cases = vec![
            ("42", Ok(("", Value::Numeric(42_f32, None)))),
            ("42%", Ok(("", Value::Numeric(42_f32, Some("%".into()))))),
            ("42px", Ok(("", Value::Numeric(42_f32, Some("px".into()))))),
        ];

        for (input, expected) in cases {
            assert_eq!(numeric(input), expected);
        }
    }

    #[test]
    fn test_number() {
        let cases = vec![
            ("1", Ok(("", 1_f32))),
            ("-1", Ok(("", -1_f32))),
            ("3.141", Ok(("", 3.141_f32))),
            ("1.5e2", Ok(("", 150_f32))),
            (".707", Ok(("", 0.707_f32))),
        ];

        for (input, expected) in cases {
            assert_eq!(number(input), expected);
        }
    }
}