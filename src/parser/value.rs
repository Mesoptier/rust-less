use nom::branch::alt;
use nom::combinator::{map, value, cut, opt};
use nom::IResult;
use nom::multi::{fold_many0, fold_many1, many1, separated_nonempty_list};
use nom::sequence::{pair, terminated, preceded};

use crate::ast::{Value, Operation, Lookup};
use crate::lexer::{symbol, ident, at_keyword, token, name, numeric};
use crate::parser::string::string;
use crate::parser::block_of_items;
use nom::bytes::complete::tag;

/// Parse a variable declaration's value
pub fn variable_declaration_value(input: &str) -> IResult<&str, Value> {
    alt((
        detached_ruleset,
        comma_list(space_list(sum_expression))
    ))(input)
}

/// Parse a declaration's value
pub fn declaration_value(input: &str) -> IResult<&str, Value> {
    comma_list(space_list(sum_expression))(input)
}

pub fn semicolon_list<'i, F>(f: F) -> impl Fn(&'i str) -> IResult<&'i str, Value<'i>>
    where F: Fn(&'i str) -> IResult<&'i str, Value<'i>>,
{
    move |input: &'i str| {
        map(
            separated_nonempty_list(symbol(";"), &f),
            |values| Value::SemicolonList(values),
        )(input)
    }
}

pub fn comma_list<'i, F>(f: F) -> impl Fn(&'i str) -> IResult<&'i str, Value<'i>>
    where F: Fn(&'i str) -> IResult<&'i str, Value<'i>>,
{
    move |input: &'i str| {
        map(
            separated_nonempty_list(symbol(","), &f),
            |values| Value::CommaList(values),
        )(input)
    }
}

pub fn space_list<'i, F>(f: F) -> impl Fn(&'i str) -> IResult<&'i str, Value<'i>>
    where F: Fn(&'i str) -> IResult<&'i str, Value<'i>>,
{
    move |input: &'i str| {
        map(
            many1(&f),
            |values| Value::SpaceList(values),
        )(input)
    }
}

fn operation_expression<'i, F, G>(operand: F, operator: G) -> impl Fn(&'i str) -> IResult<&'i str, Value<'i>>
    where
        F: Fn(&'i str) -> IResult<&'i str, Value<'i>>,
        G: Fn(&'i str) -> IResult<&'i str, Operation>,
{
    move |input: &'i str| {
        let (input, first) = operand(input)?;
        fold_many0(
            pair(&operator, &operand),
            first,
            |left, (op, right)| {
                Value::Operation(op, left.into(), right.into())
            },
        )(input)
    }
}

fn sum_expression(input: &str) -> IResult<&str, Value> {
    operation_expression(product_expression, alt((
        value(Operation::Add, symbol("+")),
        value(Operation::Subtract, symbol("-")),
    )))(input)
}

fn product_expression(input: &str) -> IResult<&str, Value> {
    operation_expression(simple_value, alt((
        value(Operation::Multiply, symbol("*")),
        value(Operation::Divide, symbol("/")),
    )))(input)
}

fn simple_value(input: &str) -> IResult<&str, Value> {
    alt((
        numeric_value,
        // color,
        string('"'),
        string('\''),
        // unicode_descriptor,
        variable_or_lookup,
        property,
        // url,
        function_call,
        // mixin_call, // includes mixin_lookup?
        ident_value,
    ))(input)
}

/// Parse a function call (e.g. `rgb(255, 0, 255)`)
fn function_call(input: &str) -> IResult<&str, Value> {
    let (input, name) = terminated(ident, symbol("("))(input)?;
    let (input, args) = function_args(input)?;
    let (input, _) = symbol(")")(input)?;
    Ok((input, Value::FunctionCall(name, Box::from(args))))
}

/// Parse a function's argument list (e.g. `(255, 0, 255)`)
fn function_args(input: &str) -> IResult<&str, Value> {
    semicolon_list(comma_list(alt((
        detached_ruleset,
        space_list(simple_value)
    ))))(input)
}

/// Parse a detached ruleset (e.g. `{ color: blue; }`)
fn detached_ruleset(input: &str) -> IResult<&str, Value> {
    let (input, block) = block_of_items(input)?;
    Ok((input, Value::DetachedRuleset(block)))
}

/// Parse a variable or variable lookup (e.g. `@var`, `@var[]`)
fn variable_or_lookup(input: &str) -> IResult<&str, Value> {
    let (input, name) = at_keyword(input)?;

    if let Ok((input, lookups)) = many1(lookup)(input) {
        return Ok((input, Value::VariableLookup(name, lookups)));
    }

    Ok((input, Value::Variable(name)))
}

/// Parse a lookup (e.g. `[]`, `[color]`, `[$@property]`)
fn lookup(input: &str) -> IResult<&str, Lookup> {
    let inner = alt((
        map(token(preceded(tag("$@"), ident)), Lookup::VariableProperty),
        map(token(preceded(tag("@@"), ident)), Lookup::VariableVariable),
        map(token(preceded(tag("$"), ident)), Lookup::Property),
        map(token(preceded(tag("@"), ident)), Lookup::Variable),
        map(token(ident), Lookup::Ident),
        value(Lookup::Last, symbol("")),
    ));
    preceded(symbol("["), terminated(cut(inner), symbol("]")))(input)
}

/// Parse a variable (e.g. `@var`)
fn variable(input: &str) -> IResult<&str, Value> {
    map(
        token(preceded(tag("@"), ident)),
        Value::Variable,
    )(input)
}

/// Parse a property accessor (e.g. `$color`)
fn property(input: &str) -> IResult<&str, Value> {
    map(
        token(preceded(tag("$"), ident)),
        Value::Property,
    )(input)
}

/// Parse a numeric value
fn numeric_value(input: &str) -> IResult<&str, Value> {
    map(
        token(numeric),
        |(value, unit)| Value::Numeric(value, unit),
    )(input)
}

/// Consume an ident value (e.g. `inherit`)
fn ident_value(input: &str) -> IResult<&str, Value> {
    map(token(ident), Value::Ident)(input)
}

#[cfg(test)]
mod tests {
    use crate::ast::{Lookup, Value};
    use crate::parser::value::{function_call, lookup, property, variable, variable_or_lookup};

    #[test]
    fn test_function_call() {
        let cases = vec![
            ("rgba(255, 0, 255)", Ok(("", Value::FunctionCall("rgba".into(), Value::CommaList(vec![
                Value::Numeric(255_f32, None),
                Value::Numeric(0_f32, None),
                Value::Numeric(255_f32, None),
            ]).into())))),
            ("repeating-linear-gradient(gold 15%, orange 30%)", Ok(("", Value::FunctionCall("repeating-linear-gradient".into(), Value::CommaList(vec![
                Value::SpaceList(vec![Value::Ident("gold".into()), Value::Numeric(15_f32, Some("%".into()))]),
                Value::SpaceList(vec![Value::Ident("orange".into()), Value::Numeric(30_f32, Some("%".into()))]),
            ]).into())))),
        ];

        for (input, expected) in cases {
            assert_eq!(function_call(input), expected);
        }
    }

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
}