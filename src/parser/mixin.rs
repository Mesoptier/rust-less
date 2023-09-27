use nom::branch::alt;
use nom::combinator::{opt, value};
use nom::multi::fold_many0;
use nom::sequence::preceded;
use nom::IResult;

use crate::ast::SimpleSelector;
use crate::lexer::{parse, symbol, token};
use crate::parser::selector::{class_selector, id_selector};

pub fn mixin_selector(input: &str) -> IResult<&str, Vec<SimpleSelector>> {
    let (input, first) = token(mixin_simple_selector)(input)?;

    token(fold_many0(
        preceded(mixin_combinator, mixin_simple_selector),
        move || vec![first.clone()],
        |mut acc, item| {
            acc.push(item);
            acc
        },
    ))(input)
}

pub fn mixin_simple_selector(input: &str) -> IResult<&str, SimpleSelector> {
    alt((id_selector, class_selector))(input)
}

/// Consume a LESS mixin combinator (e.g. ``, ` `, ` > `)
fn mixin_combinator(input: &str) -> IResult<&str, ()> {
    value((), parse(opt(symbol(">"))))(input)
}
