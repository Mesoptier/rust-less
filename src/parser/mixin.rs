use nom::branch::alt;
use nom::combinator::{opt, value};
use nom::IResult;
use nom::multi::{separated_nonempty_list, fold_many0};

use crate::ast::SimpleSelector;
use crate::lexer::{parse, symbol, token};
use crate::parser::selector::{class_selector, id_selector};
use nom::sequence::preceded;

pub fn mixin_selector(input: &str) -> IResult<&str, Vec<SimpleSelector>> {
    let (input, first) = token(mixin_simple_selector)(input)?;

    token(fold_many0(
        preceded(mixin_combinator, mixin_simple_selector),
        vec![first],
        |mut acc, item| {
            acc.push(item);
            acc
        }
    ))(input)
}

pub fn mixin_simple_selector(input: &str) -> IResult<&str, SimpleSelector> {
    alt((
        id_selector,
        class_selector,
    ))(input)
}

/// Consume a LESS mixin combinator (e.g. ``, ` `, ` > `)
fn mixin_combinator(input: &str) -> IResult<&str, ()> {
    value((), parse(opt(symbol(">"))))(input)
}
