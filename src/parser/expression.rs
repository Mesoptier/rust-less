use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{cut, map, value};
use nom::error::Error;
use nom::multi::{fold_many0, many1, separated_list1};
use nom::sequence::{pair, preceded, terminated};
use nom::{IResult, Parser};

use crate::ast::{BinaryOperator, Expression, Lookup};
use crate::lexer::{at_keyword, ident, numeric, symbol, token};
use crate::parser::block_of_items;
use crate::parser::string::string;

/// Parse a variable declaration's value
pub fn variable_declaration_value(input: &str) -> IResult<&str, Expression> {
    alt((detached_ruleset, comma_list(space_list(sum_operation))))(input)
}

/// Parse a declaration's value
pub fn declaration_value(input: &str) -> IResult<&str, Expression> {
    comma_list(space_list(sum_operation))(input)
}

pub fn comma_separated_arg_value(input: &str) -> IResult<&str, Expression> {
    space_list(sum_operation)(input)
}
pub fn semicolon_separated_arg_value(input: &str) -> IResult<&str, Expression> {
    comma_list(space_list(sum_operation))(input)
}

fn semicolon_list<'i, F>(f: F) -> impl FnMut(&'i str) -> IResult<&'i str, Expression<'i>>
where
    F: Parser<&'i str, Expression<'i>, Error<&'i str>>,
{
    map(separated_list1(symbol(";"), f), |values| {
        Expression::SemicolonList(values)
    })
}

fn comma_list<'i, F>(f: F) -> impl FnMut(&'i str) -> IResult<&'i str, Expression<'i>>
where
    F: Parser<&'i str, Expression<'i>, Error<&'i str>>,
{
    map(separated_list1(symbol(","), f), |values| {
        Expression::CommaList(values)
    })
}

fn space_list<'i, F>(f: F) -> impl FnMut(&'i str) -> IResult<&'i str, Expression<'i>>
where
    F: Parser<&'i str, Expression<'i>, Error<&'i str>>,
{
    map(many1(f), |values| Expression::SpaceList(values))
}

fn binary_operation<'i, F, G>(
    mut operand: F,
    operator: G,
) -> impl FnOnce(&'i str) -> IResult<&'i str, Expression<'i>>
where
    F: Parser<&'i str, Expression<'i>, Error<&'i str>>,
    G: Parser<&'i str, BinaryOperator, Error<&'i str>>,
{
    move |input: &'i str| {
        let (input, first) = operand.parse(input)?;
        fold_many0(
            pair(operator, operand),
            move || first.clone(),
            |left, (op, right)| Expression::BinaryOperation(op, left.into(), right.into()),
        )(input)
    }
}

fn sum_operation(input: &str) -> IResult<&str, Expression> {
    binary_operation(
        product_operation,
        alt((
            value(BinaryOperator::Add, symbol("+")),
            value(BinaryOperator::Subtract, symbol("-")),
        )),
    )(input)
}

fn product_operation(input: &str) -> IResult<&str, Expression> {
    binary_operation(
        simple_expression,
        alt((
            value(BinaryOperator::Multiply, symbol("*")),
            value(BinaryOperator::Divide, symbol("/")),
        )),
    )(input)
}

fn simple_expression(input: &str) -> IResult<&str, Expression> {
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
fn function_call(input: &str) -> IResult<&str, Expression> {
    let (input, name) = terminated(ident, symbol("("))(input)?;
    let (input, args) = function_args(input)?;
    let (input, _) = symbol(")")(input)?;
    Ok((input, Expression::FunctionCall(name, Box::from(args))))
}

/// Parse a function's argument list (e.g. `(255, 0, 255)`)
fn function_args(input: &str) -> IResult<&str, Expression> {
    semicolon_list(comma_list(alt((
        detached_ruleset,
        space_list(simple_expression),
    ))))(input)
}

/// Parse a detached ruleset (e.g. `{ color: blue; }`)
fn detached_ruleset(input: &str) -> IResult<&str, Expression> {
    let (input, block) = block_of_items(input)?;
    Ok((input, Expression::DetachedRuleset(block)))
}

/// Parse a variable or variable lookup (e.g. `@var`, `@var[]`)
fn variable_or_lookup(input: &str) -> IResult<&str, Expression> {
    let (input, name) = at_keyword(input)?;

    if let Ok((input, lookups)) = many1(lookup)(input) {
        return Ok((input, Expression::VariableLookup(name, lookups)));
    }

    Ok((input, Expression::Variable(name)))
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
fn variable(input: &str) -> IResult<&str, Expression> {
    map(token(preceded(tag("@"), ident)), Expression::Variable)(input)
}

/// Parse a property accessor (e.g. `$color`)
fn property(input: &str) -> IResult<&str, Expression> {
    map(token(preceded(tag("$"), ident)), Expression::Property)(input)
}

/// Parse a numeric value
fn numeric_value(input: &str) -> IResult<&str, Expression> {
    map(token(numeric), |(value, unit)| {
        Expression::Numeric(value, unit)
    })(input)
}

/// Consume an ident value (e.g. `inherit`)
fn ident_value(input: &str) -> IResult<&str, Expression> {
    map(token(ident), Expression::Ident)(input)
}

#[cfg(test)]
mod tests {
    use crate::ast::{Expression, Lookup};
    use crate::parser::expression::{
        function_call, lookup, property, variable, variable_or_lookup,
    };

    #[test]
    fn test_function_call() {
        let cases = vec![
            (
                "rgba(255, 0, 255)",
                Ok((
                    "",
                    Expression::FunctionCall(
                        "rgba".into(),
                        Expression::SemicolonList(vec![Expression::CommaList(vec![
                            Expression::SpaceList(vec![Expression::Numeric(255_f32, None)]),
                            Expression::SpaceList(vec![Expression::Numeric(0_f32, None)]),
                            Expression::SpaceList(vec![Expression::Numeric(255_f32, None)]),
                        ])])
                        .into(),
                    ),
                )),
            ),
            (
                "repeating-linear-gradient(gold 15%, orange 30%)",
                Ok((
                    "",
                    Expression::FunctionCall(
                        "repeating-linear-gradient".into(),
                        Expression::SemicolonList(vec![Expression::CommaList(vec![
                            Expression::SpaceList(vec![
                                Expression::Ident("gold".into()),
                                Expression::Numeric(15_f32, Some("%".into())),
                            ]),
                            Expression::SpaceList(vec![
                                Expression::Ident("orange".into()),
                                Expression::Numeric(30_f32, Some("%".into())),
                            ]),
                        ])])
                        .into(),
                    ),
                )),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(function_call(input), expected);
        }
    }

    #[test]
    fn test_variable_or_lookup() {
        let cases = vec![
            ("@var", Ok(("", Expression::Variable("var".into())))),
            (
                "@last[]",
                Ok((
                    "",
                    Expression::VariableLookup("last".into(), vec![Lookup::Last]),
                )),
            ),
            (
                "@mult[][]",
                Ok((
                    "",
                    Expression::VariableLookup("mult".into(), vec![Lookup::Last, Lookup::Last]),
                )),
            ),
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
            (
                "[@@variable]",
                Ok(("", Lookup::VariableVariable("variable".into()))),
            ),
            (
                "[$@property]",
                Ok(("", Lookup::VariableProperty("property".into()))),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(lookup(input), expected);
        }
    }

    #[test]
    fn test_variable() {
        let cases = vec![("@var", Ok(("", Expression::Variable("var".into()))))];

        for (input, expected) in cases {
            assert_eq!(variable(input), expected);
        }
    }

    #[test]
    fn test_property() {
        let cases = vec![("$color", Ok(("", Expression::Property("color".into()))))];

        for (input, expected) in cases {
            assert_eq!(property(input), expected);
        }
    }
}
